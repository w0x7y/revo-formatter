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
pub(crate) struct Analysis {
    pub tokens: Vec<SourceToken>,
    pub regions: Vec<SyntaxRegion>,
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
pub(crate) fn analyze(source: &str) -> Result<Analysis, FormatError> {
    // SAFETY: both arguments describe the borrowed source, live for this call.
    let result = decode(unsafe { revo_analyze(source.as_ptr(), source.len()) })?;
    Ok(Analysis {
        tokens: result
            .tokens
            .ok_or_else(|| FormatError::Validation("missing tokens".into()))?,
        regions: result
            .regions
            .ok_or_else(|| FormatError::Validation("missing regions".into()))?,
    })
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
            .tokens
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
        assert!(analyze("").unwrap().tokens.is_empty());
    }
    #[test]
    fn source_regions_include_real_blocks_statements_and_match_arms() {
        let source = "do let x = 1; match x | 1 => 2 | _ => 3 end";
        let analysis = analyze(source).unwrap();
        let regions: Vec<_> = analysis
            .regions
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
                !analysis.regions.iter().any(|r| r.kind == "block"),
                "{source}: {:?}",
                analysis.regions
            );
            assert!(
                analysis
                    .regions
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
            assert_eq!(analysis.tokens.len(), token_count, "{source}");
            let actual: Vec<_> = analysis
                .regions
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
            assert_eq!(analysis.tokens.len(), token_count, "{source}");
            let actual: Vec<_> = analysis
                .regions
                .iter()
                .map(|region| (region.kind.as_str(), region.start, region.end))
                .collect();
            assert_eq!(actual, expected, "{source}");
            for token in analysis.tokens {
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
            assert_eq!(analysis.tokens.len(), count * 6);
            assert_eq!(analysis.regions.len(), count * 3);
            assert_eq!(
                analysis
                    .regions
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
            assert_eq!(analysis.tokens.len(), count * 4 + 2);
            assert_eq!(analysis.regions.len(), count + 1);
            let actual: Vec<_> = analysis
                .regions
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
