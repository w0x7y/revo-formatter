use crate::FormatError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct SourceToken {
    pub kind: String,
    pub start: usize,
    pub end: usize,
}
/// Source-backed hints, not a concrete syntax tree. Statements are immediate
/// children of statement lists (including the module); blocks are concrete AST
/// blocks; match arms begin at their matcher. Spans can omit surface punctuation.
/// Tokens, never regions, remain the authority for printing original bytes.
#[derive(Debug, Deserialize)]
pub(crate) struct SyntaxRegion {
    pub kind: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Debug)]
struct Analysis {
    tokens: Vec<SourceToken>,
    regions: Vec<SyntaxRegion>,
}
/// Source text and validated metadata are inseparable at every layout boundary.
#[derive(Debug)]
pub(crate) struct AnalyzedSource<'a> {
    source: &'a str,
    analysis: Analysis,
}

impl<'a> AnalyzedSource<'a> {
    fn new(source: &'a str, analysis: Analysis) -> Result<Self, FormatError> {
        let mut previous_end = 0;
        for token in &analysis.tokens {
            if token.start < previous_end
                || token.start >= token.end
                || source.get(token.start..token.end).is_none()
            {
                return Err(FormatError::Validation(
                    "invalid frontend token range".into(),
                ));
            }
            previous_end = token.end;
        }
        for region in &analysis.regions {
            if source.get(region.start..region.end).is_none() {
                return Err(FormatError::Validation(
                    "invalid frontend region range".into(),
                ));
            }
        }
        Ok(Self { source, analysis })
    }

    fn from_response(source: &'a str, response: Response) -> Result<Self, FormatError> {
        Self::new(
            source,
            Analysis {
                tokens: response
                    .tokens
                    .ok_or_else(|| FormatError::Validation("missing tokens".into()))?,
                regions: response
                    .regions
                    .ok_or_else(|| FormatError::Validation("missing regions".into()))?,
            },
        )
    }

    pub(crate) fn source(&self) -> &'a str {
        self.source
    }
    pub(crate) fn tokens(&self) -> &[SourceToken] {
        &self.analysis.tokens
    }
    pub(crate) fn regions(&self) -> &[SyntaxRegion] {
        &self.analysis.regions
    }

    fn same_tape(&self, other: &AnalyzedSource<'_>) -> bool {
        self.tokens().len() == other.tokens().len()
            && self.tokens().iter().zip(other.tokens()).all(|(a, b)| {
                a.kind == b.kind && self.source[a.start..a.end] == other.source[b.start..b.end]
            })
    }

    pub(crate) fn preserves(&self, candidate: &str) -> Result<bool, FormatError> {
        let formatted = match analyze(candidate) {
            Ok(analysis) => analysis,
            Err(FormatError::Syntax { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        Ok(self.same_tape(&formatted) && equivalent(self.source, candidate)?)
    }
}

#[repr(C)]
struct Buffer {
    ptr: *mut u8,
    len: usize,
}
unsafe extern "C" {
    fn revo_analyze(ptr: *const u8, len: usize) -> Buffer;
    fn revo_equivalent(a: *const u8, a_len: usize, b: *const u8, b_len: usize) -> Buffer;
    fn revo_free(buffer: Buffer);
}
impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: Zig owns this allocation; only this guard releases it.
        unsafe {
            revo_free(Buffer {
                ptr: self.ptr,
                len: self.len,
            })
        };
    }
}
#[derive(Deserialize)]
struct SyntaxFailure {
    message: String,
    offset: usize,
}
#[derive(Deserialize)]
struct Response {
    syntax: Option<SyntaxFailure>,
    validation: Option<String>,
    equivalent: Option<bool>,
    tokens: Option<Vec<SourceToken>>,
    regions: Option<Vec<SyntaxRegion>>,
}
fn decode(buffer: Buffer) -> Result<Response, FormatError> {
    if buffer.ptr.is_null() || buffer.len == 0 || buffer.len > isize::MAX as usize {
        return Err(FormatError::Validation(
            "frontend returned an empty or invalid buffer".into(),
        ));
    }
    // SAFETY: the C ABI returns a live allocation of len bytes. The guard keeps
    // it alive through deserialization and frees it on success and every error.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.ptr, buffer.len) };
    decode_response(bytes)
}

fn decode_response(bytes: &[u8]) -> Result<Response, FormatError> {
    let result: Response = serde_json::from_slice(bytes)
        .map_err(|error| FormatError::Validation(format!("invalid frontend response: {error}")))?;
    if let Some(error) = result.syntax {
        return Err(FormatError::Syntax {
            message: error.message,
            offset: error.offset,
        });
    }
    if let Some(error) = result.validation {
        return Err(FormatError::Validation(error));
    }
    Ok(result)
}
pub(crate) fn analyze(source: &str) -> Result<AnalyzedSource<'_>, FormatError> {
    // SAFETY: both arguments describe the borrowed source, live for this call.
    let result = decode(unsafe { revo_analyze(source.as_ptr(), source.len()) })?;
    AnalyzedSource::from_response(source, result)
}
pub(crate) fn equivalent(original: &str, candidate: &str) -> Result<bool, FormatError> {
    // SAFETY: source slices remain live for the complete synchronous call.
    decode(unsafe {
        revo_equivalent(
            original.as_ptr(),
            original.len(),
            candidate.as_ptr(),
            candidate.len(),
        )
    })?
    .equivalent
    .ok_or_else(|| FormatError::Validation("missing equivalence result".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preservation_rejects_same_tape_different_ast() {
        for (source, candidate) in [("f(1)", "f (1)"), ("f 'hello'", "f\n'hello'")] {
            let original = analyze(source).unwrap();
            let changed = analyze(candidate).unwrap();
            assert!(original.same_tape(&changed));
            assert!(!equivalent(source, candidate).unwrap());
            assert!(!original.preserves(candidate).unwrap());
        }
    }

    #[test]
    fn preservation_rejects_equal_ast_different_tape() {
        for (source, candidate) in [
            ("let x = 1 ## note ##", "## note ## let x = 1"),
            ("let x = 'same'", "let x = \"same\""),
        ] {
            let original = analyze(source).unwrap();
            let changed = analyze(candidate).unwrap();
            assert!(equivalent(source, candidate).unwrap());
            assert!(!original.same_tape(&changed));
            assert!(!original.preserves(candidate).unwrap());
        }
    }

    #[test]
    fn preservation_accepts_layout_changes_and_rejects_invalid_candidates() {
        for (source, candidate) in [
            ("", ""),
            ("let x=1", "let x = 1\n"),
            ("# 文書\r\nlet x=\"é\\n\"", "# 文書\r\nlet x = \"é\\n\"\r\n"),
            ("let x='first\nsecond'", "let x = 'first\nsecond'\r\n"),
            ("let x=`do let y = 1 end`", "let x = `do let y = 1 end`\n"),
        ] {
            assert!(
                analyze(source).unwrap().preserves(candidate).unwrap(),
                "{source:?}"
            );
        }
        let original = analyze("let x=1").unwrap();
        for candidate in ["let x =", "\"unterminated", "let x = 1 )"] {
            assert!(!original.preserves(candidate).unwrap());
        }
        let error = analyze("let x =").unwrap_err();
        assert_eq!(
            crate::format("let x =", &crate::FormatOptions::default())
                .unwrap_err()
                .to_string(),
            error.to_string(),
        );
        assert!(
            matches!(error, FormatError::Syntax { ref message, offset } if !message.is_empty() && offset <= 7)
        );
    }

    #[test]
    fn analyzed_source_rejects_malformed_token_ranges() {
        for ranges in [
            vec![(2, 1)],
            vec![(0, 5)],
            vec![(0, 0)],
            vec![(0, 3), (2, 4)],
            vec![(3, 4), (0, 2)],
            vec![(0, 1)],
            vec![(1, 2)],
        ] {
            let analysis = Analysis {
                tokens: ranges
                    .into_iter()
                    .map(|(start, end)| SourceToken {
                        kind: "ident".into(),
                        start,
                        end,
                    })
                    .collect(),
                regions: vec![],
            };
            assert!(matches!(
                AnalyzedSource::new("é x", analysis),
                Err(FormatError::Validation(_))
            ));
        }
    }

    #[test]
    fn analyzed_source_validates_region_slices_without_ordering_them() {
        let region = |start, end| SyntaxRegion {
            kind: "statement".into(),
            start,
            end,
        };
        for (start, end) in [(3, 2), (0, 5), (0, 1), (1, 2)] {
            assert!(matches!(
                AnalyzedSource::new(
                    "é x",
                    Analysis {
                        tokens: vec![],
                        regions: vec![region(start, end)]
                    }
                ),
                Err(FormatError::Validation(_))
            ));
        }
        let analyzed = AnalyzedSource::new(
            "é x",
            Analysis {
                tokens: vec![
                    SourceToken {
                        kind: "ident".into(),
                        start: 0,
                        end: 2,
                    },
                    SourceToken {
                        kind: "ident".into(),
                        start: 3,
                        end: 4,
                    },
                ],
                regions: vec![
                    region(3, 4),
                    region(0, 4),
                    region(0, 2),
                    region(2, 2),
                    region(4, 4),
                ],
            },
        )
        .unwrap();
        assert_eq!(analyzed.source(), "é x");
        assert_eq!(analyzed.regions().len(), 5);
    }

    #[test]
    fn overlapping_statement_hints_choose_minimum_eligible_end() {
        let source = "a + b + c + d";
        let mut analyzed = analyze(source).unwrap();
        // Crossing, duplicated and out-of-order hints need no tree nesting rule.
        analyzed.analysis.regions = [(2, 7), (0, 5), (0, 5)]
            .into_iter()
            .map(|(start, end)| SyntaxRegion {
                kind: "statement".into(),
                start: analyzed.tokens()[start].start,
                end: analyzed.tokens()[end - 1].end,
            })
            .collect();
        let analyzed = AnalyzedSource::new(source, analyzed.analysis).unwrap();
        let index = crate::layout_index::LayoutIndex::new(&analyzed);
        let ends: Vec<_> = (0..analyzed.tokens().len())
            .map(|i| index.statement_end(i))
            .collect();
        assert_eq!(
            ends,
            [Some(5), Some(5), Some(5), Some(5), Some(7), Some(7), None]
        );
    }

    #[test]
    fn frontend_response_errors_remain_validation_failures() {
        for bytes in [
            b"invalid json".as_slice(),
            br#"{"validation":"OutOfMemory"}"#,
            br#"{"tokens":"bad schema"}"#,
        ] {
            assert!(matches!(
                decode_response(bytes),
                Err(FormatError::Validation(_))
            ));
        }
        for bytes in [
            br#"{}"#.as_slice(),
            br#"{"tokens":[]}"#,
            br#"{"tokens":[{"kind":"ident","start":0,"end":9}],"regions":[]}"#,
        ] {
            let response = decode_response(bytes).unwrap();
            assert!(matches!(
                AnalyzedSource::from_response("x", response),
                Err(FormatError::Validation(_))
            ));
        }
        let error = decode_response(br#"{"syntax":{"message":"original diagnostic","offset":3}}"#)
            .err()
            .unwrap();
        assert!(
            matches!(error, FormatError::Syntax { message, offset: 3 } if message == "original diagnostic")
        );
        let response = decode_response(br#"{"tokens":[],"regions":[]}"#).unwrap();
        assert!(
            AnalyzedSource::from_response("", response)
                .unwrap()
                .preserves("")
                .unwrap()
        );
    }

    #[test]
    fn null_bridge_buffers_are_validation_errors() {
        for len in [0, 1] {
            assert!(matches!(
                decode(Buffer {
                    ptr: std::ptr::null_mut(),
                    len
                }),
                Err(FormatError::Validation(_))
            ));
        }
    }
    #[test]
    fn accepts_complete_source_and_empty_input() {
        assert!(analyze("let x = 1 + 2 * 3").is_ok());
        assert!(analyze("").is_ok());
    }
    #[test]
    fn rejects_lexical_parse_and_recovery_errors() {
        for source in [
            "let x =",
            "\"unterminated",
            "let x = 1 )",
            "let = 1; let y = 2",
            "proc () 1",
        ] {
            assert!(
                matches!(analyze(source), Err(FormatError::Syntax { .. })),
                "{source}"
            );
        }
    }
    #[test]
    fn detects_precedence_and_adjacency_changes() {
        assert!(equivalent("let x = 1 + 2 * 3", "let x = 1+2*3\n").unwrap());
        for (a, b) in [
            ("let x = 1 + 2 * 3", "let x = (1 + 2) * 3"),
            ("let x = f \"hi\"", "let x = f\n\"hi\""),
            ("f(1)", "f (1)"),
            ("f<T>(1)", "f<U>(1)"),
            ("fn(x) x", "fn(?x) x"),
            ("fn(x = 1) x", "fn(x = 2) x"),
            ("fn(x: number) x", "fn(x: string) x"),
            ("let x = 1", "pub let x = 1"),
            ("let x = 1", "const x = 1"),
            ("1", "1.0"),
        ] {
            assert!(
                !equivalent(a, b).unwrap_or_else(|e| panic!("{a} vs {b}: {e}")),
                "{a} vs {b}"
            );
        }
    }
    #[test]
    fn source_tokens_preserve_comment_and_literal_envelopes() {
        let source = "#!\nmodule é\n!#\n# line\r\n## block ##\n  #*\ndoc\n*#\nlet x = \"é\\n\"";
        let analysis = analyze(source).unwrap();
        let slices: Vec<_> = analysis
            .tokens()
            .iter()
            .map(|t| (t.kind.as_str(), &source[t.start..t.end]))
            .collect();
        assert_eq!(
            slices,
            vec![
                ("module_doc", "#!\nmodule é\n!#"),
                ("comment", "# line\r"),
                ("comment", "## block ##"),
                ("doc_comment", "#*\ndoc\n*#"),
                ("kw_let", "let"),
                ("ident", "x"),
                ("assign", "="),
                ("string", "\"é\\n\"")
            ]
        );
        assert!(analyze("").unwrap().tokens().is_empty());
    }
    #[test]
    fn source_regions_include_real_blocks_statements_and_match_arms() {
        let source = "do let x = 1; match x | 1 => 2 | _ => 3 end";
        let analysis = analyze(source).unwrap();
        let regions: Vec<_> = analysis
            .regions()
            .iter()
            .map(|r| (r.kind.as_str(), &source[r.start..r.end]))
            .collect();
        assert!(regions.contains(&("block", source)), "{regions:?}");
        assert!(regions.contains(&("statement", "let x = 1")), "{regions:?}");
        assert!(regions.contains(&("match_arm", "1 => 2")), "{regions:?}");
        assert!(regions.contains(&("match_arm", "_ => 3")), "{regions:?}");
    }
    #[test]
    fn generated_blocks_and_quote_inner_nodes_are_not_source_regions() {
        for source in [
            "1 |> 2",
            "do 1 end |> 2",
            "import { \"a\", \"b\" }",
            "`do let x = 1 end`",
        ] {
            let analysis = analyze(source).unwrap();
            assert!(
                !analysis.regions().iter().any(|r| r.kind == "block"),
                "{source}: {:?}",
                analysis.regions()
            );
            assert!(
                analysis
                    .regions()
                    .iter()
                    .all(|r| r.start < r.end && r.end <= source.len())
            );
        }
    }
    #[test]
    fn source_metadata_preserves_nested_labeled_blocks_and_comment_arm_starts() {
        for (source, token_count, expected) in [
            (
                "do/outer let x = do/inner 1 end; x end",
                14,
                vec![
                    ("statement", 0, 38),
                    ("block", 0, 38),
                    ("statement", 9, 27),
                    ("block", 17, 31),
                    ("statement", 26, 27),
                    ("statement", 33, 34),
                ],
            ),
            (
                "match x | ## before ## 1 => 2 | # next\n _ => 3",
                12,
                vec![
                    ("statement", 0, 46),
                    ("match_arm", 10, 29),
                    ("match_arm", 32, 46),
                ],
            ),
        ] {
            let analysis = analyze(source).unwrap();
            assert_eq!(analysis.tokens().len(), token_count, "{source}");
            let actual: Vec<_> = analysis
                .regions()
                .iter()
                .map(|region| (region.kind.as_str(), region.start, region.end))
                .collect();
            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    fn source_metadata_keeps_opaque_and_generated_descendants_excluded() {
        for (source, token_count, expected) in [
            (
                "import { \"a\", \"b\" }; let x = 1",
                11,
                vec![("statement", 21, 30)],
            ),
            ("let x = `do let y = 1 end`", 4, vec![("statement", 0, 26)]),
            (
                "let x = \"value #{do let y = 1; y end}\"",
                4,
                vec![("statement", 0, 38)],
            ),
            ("1 |> 2", 3, vec![]),
            ("", 0, vec![]),
            (
                "#! módulo é !#\n#* 文書 *#\nlet x = \"é\"",
                6,
                vec![("statement", 30, 42)],
            ),
        ] {
            let analysis = analyze(source).unwrap();
            assert_eq!(analysis.tokens().len(), token_count, "{source}");
            let actual: Vec<_> = analysis
                .regions()
                .iter()
                .map(|region| (region.kind.as_str(), region.start, region.end))
                .collect();
            assert_eq!(actual, expected, "{source}");
            for token in analysis.tokens() {
                assert!(
                    source.get(token.start..token.end).is_some(),
                    "{source}: {token:?}"
                );
            }
        }
    }

    #[test]
    fn source_metadata_scales_to_many_statements_and_arms() {
        for count in [200, 400, 800] {
            let statements = (0..count)
                .map(|i| format!("do let x = {i} end"))
                .collect::<Vec<_>>()
                .join("\n");
            let analysis = analyze(&statements).unwrap();
            assert_eq!(analysis.tokens().len(), count * 6);
            assert_eq!(analysis.regions().len(), count * 3);
            assert_eq!(
                analysis
                    .regions()
                    .iter()
                    .filter(|r| r.kind == "block")
                    .count(),
                count
            );
            let arms = format!(
                "match x {}",
                (0..count)
                    .map(|i| format!("| {i} => {i}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let analysis = analyze(&arms).unwrap();
            assert_eq!(analysis.tokens().len(), count * 4 + 2);
            assert_eq!(analysis.regions().len(), count + 1);
            let actual: Vec<_> = analysis
                .regions()
                .iter()
                .filter(|r| r.kind == "match_arm")
                .map(|r| &arms[r.start..r.end])
                .collect();
            let expected: Vec<_> = (0..count).map(|i| format!("{i} => {i}")).collect();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn preserves_ranges_labels_docs_types_and_nested_position_policy() {
        for (a, b) in [
            ("for x in 1.. do x end", "for x in 1..3 do x end"),
            (
                "loop/one do break/one nil end",
                "loop/two do break/two nil end",
            ),
            ("#* one *# let x = 1", "#* two *# let x = 1"),
            ("type X = number", "declare X = number"),
            (
                "declare f = fn<T>(x: T) -> T",
                "declare f = fn<U>(x: U) -> U",
            ),
            ("o:f(1)", "o.f(1)"),
            (
                "declare f = fn(x: number...) -> number",
                "declare f = fn(x: number) -> number",
            ),
            ("declare X = {?x: number}", "declare X = {x: number}"),
        ] {
            assert!(
                !equivalent(a, b).unwrap_or_else(|e| panic!("{a} vs {b}: {e}")),
                "{a} vs {b}"
            );
        }
        assert!(equivalent("proc offset!(iter) do let f = iter:next(); {{:number, f[1][0][1][0]}} end; offset!(fn(x) x)", "proc offset!(iter) do let f = iter:next(); {{:number, f[1][0][1][0]}} end; offset!(fn(  x) x)").unwrap());
    }
    #[test]
    fn repeated_and_concurrent_parses_own_their_results() {
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..30 {
                        assert!(analyze("let x = fn(v: number) v").is_ok());
                        assert!(equivalent("fn(x: number) x", "fn( x : number ) x\n").unwrap());
                    }
                });
            }
        });
    }
}
