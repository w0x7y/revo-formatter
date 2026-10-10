//! Whitespace policy over the original, interleaved token tape.
use crate::{
    FormatOptions,
    document::{self, Doc, Indentation},
    layout_index::{LayoutIndex, Scope, ScopeKind},
    oracle::{AnalyzedSource, SourceToken},
};
use gaps::Join;

/// Conservative mode retains every gap's empty/nonempty and same/different-line
/// decisions. It only canonicalizes whitespace and indentation, capping blank
/// lines at the configured limit. This preserves all pinned parser whitespace
/// branches; callers still verify token bytes and the complete AST before using
/// the result.
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
        max_newlines: options.max_blank_lines + 1,
    };
    let doc = Doc::concat(vec![builder.sequence(0, builder.tokens.len()), Doc::Hard]);
    let indentation = Indentation {
        style: options.indent_style,
        columns: options.indent_width,
    };
    document::render(&doc, indentation, options.line_width, ending)
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
    max_newlines: usize,
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
            .clamp(1, self.max_newlines);
        Doc::concat((0..count).map(|_| Doc::Hard).collect())
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
                parts.push(self.whitespace(i, Join::Ordinary));
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
                        parts.push(self.whitespace(i + 1, Join::List));
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
                parts.push(self.whitespace(i, Join::Expression));
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
                    parts.push(self.whitespace(i, Join::Ordinary));
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
                            self.whitespace(body, Join::Expression),
                            Doc::Text("do"),
                        ])),
                        self.block_tail(body, close),
                    ])
                } else {
                    let leading =
                        if self.conservative || self.comment(body - 1) || self.comment(body) {
                            self.whitespace(body, Join::Ordinary)
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
                        self.whitespace(close + 1, Join::Ordinary),
                        Doc::Text(self.text(close + 1)),
                    ]));
                    self.delimited(close + 1, call_close, opening)
                } else {
                    delimiter
                }
            }
            ScopeKind::Match { arms } => Doc::concat(vec![
                self.expression_segment(start, arms),
                self.whitespace(arms, Join::Ordinary),
                self.sequence(arms, scope.end),
            ])
            .group(),
            ScopeKind::Arm {
                arrow,
                multiline_body,
            } => {
                let body = Doc::concat(vec![
                    self.whitespace(arrow + 1, Join::Ordinary),
                    self.sequence(arrow + 1, scope.end),
                ]);
                Doc::concat(vec![
                    Doc::Text(self.text(start)),
                    self.whitespace(start + 1, Join::Ordinary),
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
            return Doc::concat(vec![
                self.whitespace(close, Join::Ordinary),
                Doc::Text(self.text(close)),
            ]);
        }
        Doc::concat(vec![
            Doc::concat(vec![
                self.whitespace(open + 1, Join::Ordinary),
                self.sequence(open + 1, close),
            ])
            .indent(),
            self.whitespace(close, Join::Ordinary),
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
            self.whitespace(open + 1, Join::Ordinary)
        } else {
            Doc::Soft("")
        };
        let trailing = if self.conservative || self.comment(close - 1) {
            self.whitespace(close, Join::Ordinary)
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

/// Gap decisions. Traversal states the intention of each gap between adjacent
/// tokens; this module alone decides which original newline, comment,
/// parser-sensitive adjacency, conservative-layout and blank-line rules
/// override it, and returns the whitespace document for that gap.
mod gaps {
    use super::Builder;
    use crate::document::Doc;

    /// The caller's semantic intention for the gap before a token.
    #[derive(Clone, Copy)]
    pub(super) enum Join {
        /// Keep the original line structure, otherwise join by syntax adjacency.
        Ordinary,
        /// Join tokens within one expression; a single newline may soften.
        Expression,
        /// Follow a `,` or `;` that ends a list item or explicit statement.
        List,
    }

    /// Original facts about one gap, gathered together for every decision.
    struct Gap<'a> {
        left: &'a str,
        right: &'a str,
        /// Whether the source separated the tokens at all.
        spaced: bool,
        /// Source newlines, counted before the blank-line limit.
        newlines: usize,
        left_comment: bool,
        right_comment: bool,
    }

    impl<'a> Builder<'a> {
        /// The whitespace document for the gap before token `i`.
        pub(super) fn whitespace(&self, i: usize, join: Join) -> Doc<'a> {
            let text = self.gap(i);
            let gap = Gap {
                left: self.text(i - 1),
                right: self.text(i),
                spaced: !text.is_empty(),
                newlines: text.bytes().filter(|&b| b == b'\n').count(),
                left_comment: self.comment(i - 1),
                right_comment: self.comment(i),
            };
            match self.soft(i, &gap, join) {
                Some(flat) => Doc::Soft(flat),
                None => self.ordinary(i, &gap),
            }
        }

        // The flat text of a soft break, when the intention may reflow this
        // gap. Rules are ordered by priority.
        fn soft(&self, i: usize, gap: &Gap<'a>, join: Join) -> Option<&'static str> {
            // Conservative layout retains every original gap decision, and a
            // comment keeps its line and attachment.
            if self.conservative || gap.left_comment || gap.right_comment {
                return None;
            }
            match join {
                Join::List if gap.left == "," && gap.newlines <= 1 => Some(" "),
                // One newline within an expression may soften unless it borders
                // a statement or starts a match arm. Enclosure edges stay adjacent.
                Join::List | Join::Expression
                    if gap.newlines == 1
                        && !self.index.statement_start(i)
                        && !self.index.statement_boundary(i)
                        && !self.index.arm_start(i) =>
                {
                    let edge =
                        matches!(gap.left, "(" | "[" | "{") || matches!(gap.right, ")" | "]" | "}");
                    Some(if edge { "" } else { " " })
                }
                Join::Ordinary | Join::List | Join::Expression => None,
            }
        }

        // Original line breaks under the blank-line limit, otherwise spacing
        // by syntax adjacency.
        fn ordinary(&self, i: usize, gap: &Gap<'a>) -> Doc<'a> {
            if gap.newlines > 0 || self.line_comment(i - 1) {
                return self.breaks(i);
            }
            // The source's empty or nonempty adjacency, as one space.
            let original = if gap.spaced { " " } else { "" };
            if self.conservative {
                return Doc::Text(original);
            }
            let (left, right) = (gap.left, gap.right);
            if gap.right_comment {
                return Doc::Text(" ");
            }
            // These adjacency decisions affect parsing. Keeping all angle and range
            // gaps also protects generic lookahead and open-ended for ranges.
            if right == "/" && matches!(left, "loop" | "while" | "for" | "break" | "continue") {
                return Doc::Text(original);
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
                    || gap.left_comment))
                || matches!(left, ".." | "/")
                || right == ".."
                || self.index.generic_angle(i - 1)
                || self.index.generic_angle(i)
            {
                return Doc::Text(original);
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
                return Doc::Text(original);
            }
            Doc::Text(" ")
        }
    }
}
