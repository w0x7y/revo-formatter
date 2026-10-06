//! Complete lexical envelopes and constant-time layout queries over source hints.
use std::{cmp::Reverse, collections::BinaryHeap};

use crate::oracle::AnalyzedSource;

/// A complete construct contained in the caller's token range.
#[derive(Clone, Copy)]
pub(crate) struct Scope {
    pub(crate) end: usize,
    pub(crate) kind: ScopeKind,
}

#[derive(Clone, Copy)]
pub(crate) enum ScopeKind {
    Header {
        body: usize,
        block_close: Option<usize>,
    },
    Delimited {
        close: usize,
        call_close: Option<usize>,
    },
    Match {
        arms: usize,
    },
    Arm {
        arrow: usize,
        multiline_body: bool,
    },
}

pub(crate) struct LayoutIndex {
    delimiter_closes: Vec<Option<usize>>,
    arms: Vec<Option<Scope>>,
    matches: Vec<Option<Scope>>,
    statement_ends: Vec<Option<usize>>,
    generic_angles: Vec<bool>,
    generic_closes: Vec<Option<usize>>,
    generic_calls: Vec<Option<usize>>,
    statement_starts: Vec<bool>,
    statement_boundaries: Vec<bool>,
    headers: Vec<Option<Scope>>,
    unary_signs: Vec<bool>,
}

impl LayoutIndex {
    pub(crate) fn new(analysis: &AnalyzedSource<'_>) -> Self {
        let source = analysis.source();
        let tokens = analysis.tokens();
        let text = |i: usize| &source[tokens[i].start..tokens[i].end];
        let mut index = Self {
            delimiter_closes: vec![None; tokens.len()],
            arms: vec![None; tokens.len()],
            matches: vec![None; tokens.len()],
            statement_ends: vec![None; tokens.len()],
            generic_angles: vec![false; tokens.len()],
            generic_closes: vec![None; tokens.len()],
            generic_calls: vec![None; tokens.len()],
            statement_starts: vec![false; tokens.len()],
            statement_boundaries: vec![false; tokens.len() + 1],
            headers: vec![None; tokens.len()],
            unary_signs: vec![false; tokens.len()],
        };
        let mut stack = Vec::new();
        for i in 0..tokens.len() {
            match text(i) {
                "(" | "{" | "[" => stack.push(i),
                ")" | "}" | "]" => {
                    if let Some(open) = stack.pop()
                        && matches!((text(open), text(i)), ("(", ")") | ("{", "}") | ("[", "]"))
                    {
                        index.delimiter_closes[open] = Some(i);
                    }
                }
                _ => {}
            }
        }
        // Block ownership comes from actual source-backed AST blocks. Keywords
        // are also valid identifiers, so only punctuation is paired lexically.
        for region in analysis.regions().iter().filter(|r| r.kind == "block") {
            let start = tokens.partition_point(|t| t.start < region.start);
            let end = tokens.partition_point(|t| t.start < region.end);
            if start < end
                && tokens[start].start == region.start
                && tokens[end - 1].end == region.end
                && text(start) == "do"
                && text(end - 1) == "end"
            {
                index.delimiter_closes[start] = Some(end - 1);
            }
        }
        // Preserve generic-call lookahead, including spaced receivers which
        // deliberately parse as comparisons. Match the parser's bounded shape.
        for open in 0..tokens.len() {
            if text(open) != "<" {
                continue;
            }
            for (close, token) in tokens
                .iter()
                .enumerate()
                .take((open + 33).min(tokens.len()))
                .skip(open + 1)
            {
                if text(close) == ">" {
                    let call_shape = close + 1 < tokens.len()
                        && text(close + 1) == "("
                        && tokens[close].end == tokens[close + 1].start;
                    let mut head = open.saturating_sub(1);
                    while head >= 2 && text(head - 1) == "." && tokens[head - 2].kind == "ident" {
                        head -= 2;
                    }
                    let declaration =
                        head > 0 && matches!(text(head - 1), "fn" | "type" | "declare");
                    if call_shape || declaration {
                        index.generic_angles[open] = true;
                        index.generic_angles[close] = true;
                    }
                    if declaration
                        || (call_shape
                            && open > 0
                            && tokens[open - 1].kind == "ident"
                            && tokens[open - 1].end == tokens[open].start)
                    {
                        index.generic_closes[open] = Some(close);
                        if close + 1 < tokens.len() && text(close + 1) == "(" {
                            index.generic_calls[open] = index.delimiter_closes[close + 1];
                        }
                    }
                    break;
                }
                if token.kind != "ident" && text(close) != "," {
                    break;
                }
            }
        }

        let mut statements = Vec::new();
        let mut bodies = vec![None; tokens.len()];
        let mut headers = Vec::new();
        let mut arm_ends = vec![None; tokens.len()];
        let mut arm_heads = Vec::new();
        let mut match_ends = vec![None; tokens.len()];
        let mut match_heads = Vec::new();
        for region in analysis.regions() {
            let start = tokens.partition_point(|t| t.start < region.start);
            let end = tokens.partition_point(|t| t.start < region.end);
            if start >= tokens.len() {
                continue;
            }
            match region.kind.as_str() {
                "unary_sign" => {
                    index.unary_signs[start] = text(start) == "-";
                    continue;
                }
                "header" => {
                    headers.push((start, end));
                    continue;
                }
                "body" => {
                    bodies[start] = Some(index.envelope_end(start, end));
                    continue;
                }
                "match_expression" => {
                    match_ends[start] = Some(index.envelope_end(start, end));
                    continue;
                }
                "match_head" => {
                    match_heads.push((start, end));
                    continue;
                }
                "match_arm_head" => {
                    if start > 0 && text(start - 1) == "|" && end > start && text(end - 1) == "=>" {
                        arm_heads.push((start - 1, end - 1));
                    }
                    continue;
                }
                "statement" | "match_arm" => {}
                _ => continue,
            }
            let end = index.envelope_end(start, end);
            if region.kind == "statement" {
                index.statement_starts[start] = true;
                index.statement_boundaries[end] = true;
                statements.push((start, end));
            } else if start > 0
                && tokens.get(start).is_some_and(|t| t.start == region.start)
                && text(start - 1) == "|"
            {
                arm_ends[start - 1] = Some(end);
            }
        }
        for (start, arms) in match_heads {
            if let Some(end) = match_ends[start].filter(|&end| end > arms && arms > start) {
                index.matches[start] = Some(Scope {
                    end,
                    kind: ScopeKind::Match { arms },
                });
            }
        }
        for (bar, arrow) in arm_heads {
            let Some(end) = arm_ends[bar].filter(|&end| end > arrow + 1) else {
                continue;
            };
            let first_code = (arrow + 1..end)
                .find(|&i| {
                    !matches!(
                        tokens[i].kind.as_str(),
                        "comment" | "doc_comment" | "module_doc"
                    )
                })
                .unwrap_or(arrow + 1);
            let multiline_body = source[tokens[arrow].end..tokens[first_code].start].contains('\n');
            index.arms[bar] = Some(Scope {
                end,
                kind: ScopeKind::Arm {
                    arrow,
                    multiline_body,
                },
            });
        }
        for (start, body) in headers {
            if body >= tokens.len() {
                continue;
            }
            let Some(mut end) = bodies[body] else {
                continue;
            };
            let mut adjusted_body = body;
            let mut cursor = start;
            while cursor < body {
                if let Some(close) = index.delimiter_close(cursor) {
                    if close >= body {
                        adjusted_body = cursor;
                        end = end.max(close + 1);
                        break;
                    }
                    cursor = close + 1;
                } else {
                    cursor += 1;
                }
            }
            let block_close = (text(adjusted_body) == "do")
                .then(|| index.delimiter_closes[adjusted_body])
                .flatten();
            // A body AST can be `do ... end + suffix`. Printing the concrete
            // block owns only its closer; the surrounding traversal keeps the
            // suffix rather than advancing past tokens it has not printed.
            index.headers[start] = Some(Scope {
                end: block_close.map_or(end, |close| close + 1),
                kind: ScopeKind::Header {
                    body: adjusted_body,
                    block_close,
                },
            });
        }
        // Source hints can overlap without nesting. At each token retain the
        // minimum end among every started region that still contains an RHS.
        statements.sort_unstable();
        let mut statements = statements.into_iter().peekable();
        let mut active = BinaryHeap::new();
        for (token, end) in index.statement_ends.iter_mut().enumerate() {
            while let Some(&(start, stop)) = statements.peek()
                && start <= token
            {
                active.push(Reverse(stop));
                statements.next();
            }
            while active
                .peek()
                .is_some_and(|&Reverse(stop)| stop <= token + 1)
            {
                active.pop();
            }
            *end = active.peek().map(|&Reverse(stop)| stop);
        }
        index
    }

    // An opener inside the hint owns its closer, even when the AST span omits
    // it. Jump complete pairs: nested delimiters cannot extend past that close.
    // A closer whose opener precedes start cannot extend this envelope.
    fn envelope_end(&self, start: usize, mut end: usize) -> usize {
        let mut cursor = start;
        while cursor < end {
            if let Some(close) = self.delimiter_close(cursor) {
                end = end.max(close + 1);
                cursor = close + 1;
            } else {
                cursor += 1;
            }
        }
        end
    }

    /// Own scope priority and containment here so every traversal skips the
    /// same lexical envelope, including a match used as an operator operand.
    pub(crate) fn scope(&self, token: usize, limit: usize) -> Option<Scope> {
        if let Some(header) = self.headers[token].filter(|scope| scope.end <= limit) {
            return Some(header);
        }
        if let Some(close) = self.delimiter_closes[token].or(self.generic_closes[token])
            && close < limit
        {
            // Generic arguments and their adjacent call share suffix fitting.
            let call_close = self.generic_calls[token].filter(|&end| end < limit);
            return Some(Scope {
                end: call_close.unwrap_or(close) + 1,
                kind: ScopeKind::Delimited { close, call_close },
            });
        }
        if let Some(scope) = self.matches[token].filter(|scope| scope.end <= limit) {
            return Some(scope);
        }
        self.arms[token].filter(|scope| scope.end > token && scope.end <= limit)
    }

    fn delimiter_close(&self, open: usize) -> Option<usize> {
        self.delimiter_closes[open]
    }

    pub(crate) fn arm_start(&self, token: usize) -> bool {
        self.arms[token].is_some()
    }

    pub(crate) fn statement_end(&self, token: usize) -> Option<usize> {
        self.statement_ends[token]
    }

    pub(crate) fn generic_angle(&self, token: usize) -> bool {
        self.generic_angles[token]
    }
    pub(crate) fn statement_start(&self, token: usize) -> bool {
        self.statement_starts[token]
    }
    pub(crate) fn statement_boundary(&self, token: usize) -> bool {
        self.statement_boundaries[token]
    }
    pub(crate) fn unary_sign(&self, token: usize) -> bool {
        self.unary_signs[token]
    }
}
