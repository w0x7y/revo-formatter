//! Whitespace policy over the original, interleaved token tape.
use crate::{
    FormatOptions,
    document::{self, Doc},
    layout_index::{LayoutIndex, Scope, ScopeKind},
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
        if self.comment(i) {
            return Doc::Text(" ");
        }
        // These adjacency decisions affect parsing. Keeping all angle and range
        // gaps also protects generic lookahead and open-ended for ranges.
        if right == "/" && matches!(left, "loop" | "while" | "for" | "break" | "continue") {
            return Doc::Text(if gap.is_empty() { "" } else { " " });
        }
        if self.index.unary_sign(i - 1) || right == "[" {
            return Doc::Text("");
        }
        if (right == "("
            && (matches!(
                self.tokens[i - 1].kind.as_str(),
                "ident"
                    | "number"
                    | "string"
                    | "multiline_string"
                    | "backtick_string"
                    | "atom"
                    | "rparen"
                    | "rbracket"
                    | "rsquiggly"
                    | "bang"
            ) || left == "fn"
                || self.comment(i - 1)))
            || matches!(left, ".." | "/")
            || right == ".."
            || self.index.generic_angle(i - 1)
            || self.index.generic_angle(i)
        {
            return Doc::Text(if gap.is_empty() { "" } else { " " });
        }
        if left == "," {
            return Doc::Text(" ");
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
            if i > segment
                && (self.index.statement_boundary(i)
                    || (self.index.statement_start(i) && self.gap(i).contains('\n')))
            {
                parts.push(self.expression_segment(segment, i));
                parts.push(self.separator(i));
                segment = i;
            }
            if let Some(scope) = self.index.scope(i, end) {
                i = scope.end;
            } else {
                // Adjacent list items and explicit statements own independent
                // continuations. Keep punctuation with the preceding item.
                if matches!(self.text(i), "," | ";") {
                    parts.push(self.expression_segment(segment, i + 1));
                    if i + 1 < end {
                        parts.push(self.list_separator(i + 1));
                    }
                    segment = i + 1;
                }
                i += 1;
            }
        }
        if segment < end {
            parts.push(self.expression_segment(segment, end));
        }
        Doc::concat(parts).group()
    }
    // Operators own independent chunks. The renderer fills available columns,
    // while delimiters, bodies, and match arms establish their own scopes.
    fn expression_segment(&self, start: usize, end: usize) -> Doc<'a> {
        let mut parts = Vec::new();
        let mut i = start;
        while i < end {
            if i > start {
                parts.push(self.expression_separator(i));
            }
            if let Some(scope) = self.index.scope(i, end) {
                parts.push(self.scope_document(i, scope));
                i = scope.end;
            } else if !self.conservative
                && self.binary(i)
                && self.index.statement_end(i).unwrap_or(end).min(end) > i + 1
            {
                parts.push(Doc::Text(self.text(i)));
                i += 1;
                // A trailing comment belongs to the operator's line, never to
                // a new continuation. Its following newline remains mandatory.
                while i < end && self.comment(i) && !self.gap(i).contains('\n') {
                    parts.push(self.separator(i));
                    parts.push(Doc::Text(self.text(i)));
                    i += 1;
                }
                return Doc::fill(Doc::concat(parts).group(), self.operand_chunks(i, end)).group();
            } else {
                parts.push(Doc::Text(self.text(i)));
                i += 1;
            }
        }
        Doc::concat(parts).group()
    }
    fn scope_document(&self, start: usize, scope: Scope) -> Doc<'a> {
        match scope.kind {
            ScopeKind::Header { body, block_close } => {
                let head = self.expression_segment(start, body);
                if let Some(close) = block_close {
                    Doc::concat(vec![
                        head.followed_by(Doc::concat(vec![
                            self.expression_separator(body),
                            Doc::Text("do"),
                        ])),
                        self.block_tail(body, close),
                    ])
                } else {
                    let leading =
                        if self.conservative || self.comment(body - 1) || self.comment(body) {
                            self.separator(body)
                        } else {
                            Doc::Soft(" ")
                        };
                    Doc::concat(vec![
                        head,
                        Doc::concat(vec![leading, self.sequence(body, scope.end)]).indent(),
                    ])
                    .group()
                }
            }
            ScopeKind::Delimited { close, call_close } => {
                let delimiter = self.delimited(start, close, Doc::Text(self.text(start)));
                if let Some(call_close) = call_close {
                    // Only the call opener must hug the generic list. Its
                    // arguments own their breaks and do not reserve columns in
                    // the generic list's independent fit decision.
                    let opening = delimiter.followed_by(Doc::concat(vec![
                        self.separator(close + 1),
                        Doc::Text(self.text(close + 1)),
                    ]));
                    self.delimited(close + 1, call_close, opening)
                } else {
                    delimiter
                }
            }
            ScopeKind::Match { arms } => Doc::concat(vec![
                self.expression_segment(start, arms),
                self.separator(arms),
                self.sequence(arms, scope.end),
            ])
            .group(),
            ScopeKind::Arm {
                arrow,
                multiline_body,
            } => {
                let body = Doc::concat(vec![
                    self.separator(arrow + 1),
                    self.sequence(arrow + 1, scope.end),
                ]);
                Doc::concat(vec![
                    Doc::Text(self.text(start)),
                    self.separator(start + 1),
                    self.sequence(start + 1, arrow + 1),
                    if multiline_body { body.indent() } else { body },
                ])
                .indent()
            }
        }
    }
    fn operand_chunks(&self, start: usize, end: usize) -> Vec<Doc<'a>> {
        let mut chunks = Vec::new();
        let mut chunk_start = start;
        let mut i = start;
        while i < end {
            if let Some(scope) = self.index.scope(i, end) {
                i = scope.end;
                continue;
            }
            if self.binary(i) && i + 1 < end {
                let mut stop = i + 1;
                while stop < end && self.comment(stop) && !self.gap(stop).contains('\n') {
                    stop += 1;
                }
                chunks.push(self.operand_chunk(chunk_start, stop));
                chunk_start = stop;
                i = stop;
            } else {
                i += 1;
            }
        }
        if chunk_start < end {
            chunks.push(self.operand_chunk(chunk_start, end));
        }
        chunks
    }
    fn operand_chunk(&self, start: usize, end: usize) -> Doc<'a> {
        let leading = if self.line_comment(start - 1)
            || (self.comment(start - 1) && self.gap(start).contains('\n'))
        {
            self.breaks(start)
        } else {
            Doc::Soft(" ")
        };
        Doc::concat(vec![leading, self.expression_segment(start, end)]).group()
    }
    fn comment(&self, i: usize) -> bool {
        matches!(
            self.tokens[i].kind.as_str(),
            "comment" | "doc_comment" | "module_doc"
        )
    }
    fn expression_separator(&self, i: usize) -> Doc<'a> {
        if !self.conservative
            && self.gap(i).contains('\n')
            && !self.index.statement_start(i)
            && !self.index.statement_boundary(i)
            && !self.index.arm_start(i)
            && !self.comment(i - 1)
            && !self.comment(i)
            && self.gap(i).bytes().filter(|&b| b == b'\n').count() == 1
        {
            let left = self.text(i - 1);
            let right = self.text(i);
            return Doc::Soft(
                if matches!(left, "(" | "[" | "{") || matches!(right, ")" | "]" | "}") {
                    ""
                } else {
                    " "
                },
            );
        }
        self.separator(i)
    }
    fn list_separator(&self, i: usize) -> Doc<'a> {
        if !self.conservative
            && self.text(i - 1) == ","
            && !self.comment(i - 1)
            && !self.comment(i)
            && self.gap(i).bytes().filter(|&b| b == b'\n').count() <= 1
        {
            Doc::Soft(" ")
        } else {
            self.expression_separator(i)
        }
    }
    fn binary(&self, i: usize) -> bool {
        i > 0
            && !self.index.unary_sign(i)
            && !self.index.generic_angle(i)
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
                    | "<"
                    | ">"
                    | "%"
                    | "^"
                    | "++"
                    | "|>"
            )
            && matches!(
                self.tokens[i - 1].kind.as_str(),
                "ident"
                    | "number"
                    | "string"
                    | "multiline_string"
                    | "atom"
                    | "rparen"
                    | "rbracket"
                    | "rsquiggly"
                    | "kw_end"
            )
    }
    fn block_tail(&self, open: usize, close: usize) -> Doc<'a> {
        if close == open + 1 {
            return Doc::concat(vec![self.separator(close), Doc::Text(self.text(close))]);
        }
        Doc::concat(vec![
            Doc::concat(vec![
                self.separator(open + 1),
                self.sequence(open + 1, close),
            ])
            .indent(),
            self.separator(close),
            Doc::Text(self.text(close)),
        ])
    }
    fn delimited(&self, open: usize, close: usize, opening: Doc<'a>) -> Doc<'a> {
        if self.text(open) == "do" {
            return Doc::concat(vec![opening, self.block_tail(open, close)]);
        }
        if close == open + 1 {
            return opening.followed_by(Doc::Text(self.text(close)));
        }
        let leading = if self.conservative || self.comment(open + 1) || self.comment(open) {
            self.separator(open + 1)
        } else {
            Doc::Soft("")
        };
        let trailing = if self.conservative || self.comment(close - 1) {
            self.separator(close)
        } else {
            Doc::Soft("")
        };
        Doc::enclosed(
            opening,
            leading,
            self.sequence(open + 1, close),
            trailing,
            self.text(close),
        )
    }
}
