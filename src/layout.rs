//! Whitespace policy over the original, interleaved token tape.
use crate::{
    FormatOptions,
    document::{self, Doc},
    layout_index::LayoutIndex,
    oracle::{AnalyzedSource, SourceToken},
};

/// Conservative mode retains every gap's empty/nonempty and same/different-line
/// decisions. It only canonicalizes whitespace and indentation, capping blank
/// lines at one. This preserves all pinned parser whitespace branches; callers
/// still verify token bytes and the complete AST before using the result.
pub(crate) fn layout(
    analysis: &AnalyzedSource<'_>,
    options: &FormatOptions,
    conservative: bool,
) -> String {
    let source = analysis.source();
    let ending = line_ending(source, analysis.tokens());
    if analysis.tokens().is_empty() {
        return if source.is_empty() {
            String::new()
        } else {
            ending.to_owned()
        };
    }
    let builder = Builder {
        source,
        tokens: analysis.tokens(),
        index: LayoutIndex::new(analysis),
        conservative,
    };
    let doc = Doc::concat(vec![builder.sequence(0, builder.tokens.len()), Doc::Hard]);
    document::render(&doc, options.indent_width, options.line_width, ending)
}

// Select the first layout newline, ignoring newline bytes inside opaque tokens.
// The preceding CR may belong to a line-comment token rather than the gap.
fn line_ending(source: &str, tokens: &[SourceToken]) -> &'static str {
    let mut start = 0;
    for (end, next_start) in tokens
        .iter()
        .map(|t| (t.start, t.end))
        .chain(std::iter::once((source.len(), source.len())))
    {
        if let Some(relative) = source[start..end].find('\n') {
            let offset = start + relative;
            return if offset > 0 && source.as_bytes()[offset - 1] == b'\r' {
                "\r\n"
            } else {
                "\n"
            };
        }
        start = next_start;
    }
    "\n"
}

struct Builder<'a> {
    source: &'a str,
    tokens: &'a [SourceToken],
    index: LayoutIndex,
    conservative: bool,
}
impl<'a> Builder<'a> {
    fn text(&self, i: usize) -> &'a str {
        &self.source[self.tokens[i].start..self.tokens[i].end]
    }
    fn gap(&self, i: usize) -> &'a str {
        &self.source[self.tokens[i - 1].end..self.tokens[i].start]
    }
    fn line_comment(&self, i: usize) -> bool {
        self.tokens[i].kind == "comment" && !self.text(i).starts_with("##")
    }
    fn breaks(&self, i: usize) -> Doc<'a> {
        let count = self
            .gap(i)
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            .clamp(1, 2);
        Doc::concat((0..count).map(|_| Doc::Hard).collect())
    }
    fn separator(&self, i: usize) -> Doc<'a> {
        let gap = self.gap(i);
        if gap.contains('\n') || self.line_comment(i - 1) {
            return self.breaks(i);
        }
        if self.conservative {
            return Doc::Text(if gap.is_empty() { "" } else { " " });
        }
        let left = self.text(i - 1);
        let right = self.text(i);
        // These adjacency decisions affect parsing. Keeping all angle and range
        // gaps also protects generic lookahead and open-ended for ranges.
        if right == "/" && matches!(left, "loop" | "while" | "for" | "break" | "continue") {
            return Doc::Text(if gap.is_empty() { "" } else { " " });
        }
        if right == "("
            || matches!(left, ".." | "/")
            || right == ".."
            || self.index.generic_angle(i - 1)
            || self.index.generic_angle(i)
        {
            return Doc::Text(if gap.is_empty() { "" } else { " " });
        }
        if left == "," {
            return Doc::Soft(" ");
        }
        if matches!(right, "," | ";" | ")" | "]" | "}") || matches!(left, "(" | "[" | "{") {
            return Doc::Text("");
        }
        if right == ":" || left == "." || right == "." {
            return Doc::Text("");
        }
        if matches!(left, "?" | "!")
            || matches!(right, "?" | "!")
            || self.tokens[i].kind == "atom"
            || self.tokens[i - 1].kind == "atom"
        {
            return Doc::Text(if gap.is_empty() { "" } else { " " });
        }
        Doc::Text(" ")
    }
    fn sequence(&self, start: usize, end: usize) -> Doc<'a> {
        let mut parts = Vec::new();
        let mut segment = start;
        let mut i = start;
        while i < end {
            if let Some(close) = self.index.delimiter_close(i).filter(|close| *close < end) {
                i = close + 1;
            } else if let Some(stop) = self
                .index
                .arm_end(i)
                .filter(|stop| *stop <= end && *stop > i)
            {
                i = stop;
            } else {
                // Adjacent list items and explicit statements own independent
                // continuations. Keep punctuation with the preceding item.
                if matches!(self.text(i), "," | ";") {
                    parts.push(self.expression_segment(segment, i + 1, false));
                    if i + 1 < end {
                        parts.push(self.separator(i + 1));
                    }
                    segment = i + 1;
                }
                i += 1;
            }
        }
        if segment < end {
            parts.push(self.expression_segment(segment, end, false));
        }
        Doc::concat(parts).group()
    }
    // Only recursive binary suffixes share a continuation level. Delimiters and
    // arm bodies call sequence and establish their own expression scope.
    fn expression_segment(&self, start: usize, end: usize, continuation: bool) -> Doc<'a> {
        let mut parts = Vec::new();
        let mut i = start;
        while i < end {
            if i > start {
                parts.push(self.separator(i));
            }
            if let Some(close) = self.index.delimiter_close(i).filter(|close| *close < end) {
                parts.push(self.delimited(i, close));
                i = close + 1;
            } else if let Some(arm_end) = self
                .index
                .arm_end(i)
                .filter(|stop| *stop <= end && *stop > i)
            {
                let arrow = (i + 1..arm_end).find(|&j| self.text(j) == "=>");
                let mut arm = vec![Doc::Text(self.text(i))];
                if let Some(arrow) = arrow {
                    arm.push(self.separator(i + 1));
                    arm.push(self.sequence(i + 1, arrow + 1));
                    if arrow + 1 < arm_end {
                        let body = Doc::concat(vec![
                            self.separator(arrow + 1),
                            self.sequence(arrow + 1, arm_end),
                        ]);
                        // Inline bodies share the arm's base indentation; their
                        // delimiters indent the contents. A body starting on the
                        // next source line receives its own continuation level.
                        let first_code = (arrow + 1..arm_end)
                            .find(|&j| {
                                !matches!(self.tokens[j].kind.as_str(), "comment" | "doc_comment")
                            })
                            .unwrap_or(arrow + 1);
                        let before_body =
                            &self.source[self.tokens[arrow].end..self.tokens[first_code].start];
                        arm.push(if before_body.contains('\n') {
                            body.indent()
                        } else {
                            body
                        });
                    }
                } else if i + 1 < arm_end {
                    arm.push(self.separator(i + 1));
                    arm.push(self.sequence(i + 1, arm_end));
                }
                parts.push(Doc::concat(arm).indent());
                i = arm_end;
            } else if !self.conservative && self.binary(i) && i + 1 < end {
                parts.push(Doc::Text(self.text(i)));
                // Limit continuations to a source-backed statement. Parentheses
                // and lists further bound the range in recursive calls.
                let stop = self.index.statement_end(i).unwrap_or(end).min(end);
                let separator = if self.gap(i + 1).contains('\n') {
                    self.breaks(i + 1)
                } else {
                    Doc::Soft(" ")
                };
                // Fit this continuation against the columns remaining after
                // its operator, independently of hard breaks in other statements.
                let suffix =
                    Doc::concat(vec![separator, self.expression_segment(i + 1, stop, true)]);
                parts.push(
                    if continuation {
                        suffix
                    } else {
                        suffix.indent()
                    }
                    .group(),
                );
                i = stop;
            } else {
                parts.push(Doc::Text(self.text(i)));
                i += 1;
            }
        }
        Doc::concat(parts).group()
    }
    fn binary(&self, i: usize) -> bool {
        i > 0
            && matches!(
                self.text(i),
                "+" | "-"
                    | "*"
                    | "/"
                    | "//"
                    | "=="
                    | "!="
                    | "<="
                    | ">="
                    | "and"
                    | "or"
                    | "orelse"
                    | "band"
                    | "bor"
                    | "bxor"
                    | "shl"
                    | "shr"
            )
            && matches!(
                self.tokens[i - 1].kind.as_str(),
                "ident" | "number" | "string" | "rparen" | "rbracket"
            )
    }
    fn delimited(&self, open: usize, close: usize) -> Doc<'a> {
        let mut parts = vec![Doc::Text(self.text(open))];
        if close == open + 1 {
            parts.push(Doc::Text(self.text(close)));
            return Doc::concat(parts);
        }
        let block = self.text(open) == "do";
        let leading = if self.conservative || block || self.gap(open + 1).contains('\n') {
            self.separator(open + 1)
        } else {
            Doc::Soft("")
        };
        parts.push(Doc::concat(vec![leading, self.sequence(open + 1, close)]).indent());
        parts.push(
            if self.conservative
                || block
                || self.gap(close).contains('\n')
                || self.line_comment(close - 1)
            {
                self.separator(close)
            } else {
                Doc::Soft("")
            },
        );
        parts.push(Doc::Text(self.text(close)));
        Doc::concat(parts).group()
    }
}
