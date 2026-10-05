//! Whitespace policy over the original, interleaved token tape.
use crate::{
    FormatOptions,
    document::{self, Doc},
    oracle::{Analysis, SourceToken},
};

/// Conservative mode retains every gap's empty/nonempty and same/different-line
/// decisions. It only canonicalizes whitespace and indentation, capping blank
/// lines at one. This preserves all pinned parser whitespace branches; callers
/// still verify token bytes and the complete AST before using the result.
pub(crate) fn layout(
    source: &str,
    analysis: &Analysis,
    options: &FormatOptions,
    conservative: bool,
) -> String {
    let ending = line_ending(source, &analysis.tokens);
    if analysis.tokens.is_empty() {
        return if source.is_empty() {
            String::new()
        } else {
            ending.to_owned()
        };
    }
    let mut builder = Builder {
        source,
        tokens: &analysis.tokens,
        pairs: vec![None; analysis.tokens.len()],
        arms: vec![None; analysis.tokens.len()],
        statements: Vec::new(),
        generic_angles: vec![false; analysis.tokens.len()],
        conservative,
    };
    let mut stack = Vec::new();
    for i in 0..builder.tokens.len() {
        match builder.text(i) {
            "(" | "{" | "[" | "do" => stack.push(i),
            ")" | "}" | "]" | "end" => {
                if let Some(open) = stack.pop()
                    && matches!(
                        (builder.text(open), builder.text(i)),
                        ("(", ")") | ("{", "}") | ("[", "]") | ("do", "end")
                    )
                {
                    builder.pairs[open] = Some(i);
                }
            }
            _ => {}
        }
    }
    // Preserve generic-call lookahead shapes, including nonadjacent receivers
    // such as `f <T>(x)` which deliberately parse as comparisons.
    for open in 0..builder.tokens.len() {
        if builder.text(open) != "<" {
            continue;
        }
        for close in open + 1..(open + 33).min(builder.tokens.len()) {
            if builder.text(close) == ">" {
                builder.generic_angles[open] = true;
                builder.generic_angles[close] = true;
                break;
            }
            if builder.tokens[close].kind != "ident" && builder.text(close) != "," {
                break;
            }
        }
    }
    // Regions bound statements and identify match arms; tokens own every byte.
    for region in &analysis.regions {
        if region.kind == "statement" {
            let start = builder.tokens.partition_point(|t| t.start < region.start);
            let end = builder.tokens.partition_point(|t| t.start < region.end);
            builder.statements.push((start, end));
        }
        if region.kind == "match_arm"
            && let Some(start) = builder.tokens.iter().position(|t| t.start == region.start)
            && start > 0
            && builder.text(start - 1) == "|"
        {
            let end = builder.tokens.partition_point(|t| t.start < region.end);
            builder.arms[start - 1] = Some(end);
        }
    }
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
    pairs: Vec<Option<usize>>,
    arms: Vec<Option<usize>>,
    statements: Vec<(usize, usize)>,
    generic_angles: Vec<bool>,
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
            || self.generic_angles[i - 1]
            || self.generic_angles[i]
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
        let mut i = start;
        while i < end {
            if i > start {
                parts.push(self.separator(i));
            }
            if let Some(close) = self.pairs[i].filter(|close| *close < end) {
                parts.push(self.delimited(i, close));
                i = close + 1;
            } else if let Some(arm_end) = self.arms[i].filter(|stop| *stop <= end && *stop > i) {
                let arrow = (i + 1..arm_end).find(|&j| self.text(j) == "=>");
                let mut arm = vec![Doc::Text(self.text(i))];
                if let Some(arrow) = arrow {
                    arm.push(self.separator(i + 1));
                    arm.push(self.sequence(i + 1, arrow + 1));
                    if arrow + 1 < arm_end {
                        arm.push(
                            Doc::concat(vec![
                                self.separator(arrow + 1),
                                self.sequence(arrow + 1, arm_end),
                            ])
                            .indent(),
                        );
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
                let stop = self
                    .statements
                    .iter()
                    .filter(|&&(a, b)| a <= i && b > i + 1)
                    .map(|&(_, b)| b)
                    .min()
                    .unwrap_or(end)
                    .min(end);
                let separator = if self.gap(i + 1).contains('\n') {
                    self.breaks(i + 1)
                } else {
                    Doc::Soft(" ")
                };
                // Fit this continuation against the columns remaining after
                // its operator, independently of hard breaks in other statements.
                parts.push(
                    Doc::concat(vec![separator, self.sequence(i + 1, stop)])
                        .indent()
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
