//! Complete lexical envelopes and constant-time layout queries over source hints.
use std::{cmp::Reverse, collections::BinaryHeap};

use crate::oracle::AnalyzedSource;

pub(crate) struct LayoutIndex {
    delimiter_closes: Vec<Option<usize>>,
    arm_ends: Vec<Option<usize>>,
    statement_ends: Vec<Option<usize>>,
    generic_angles: Vec<bool>,
}

impl LayoutIndex {
    pub(crate) fn new(analysis: &AnalyzedSource<'_>) -> Self {
        let source = analysis.source();
        let tokens = analysis.tokens();
        let text = |i: usize| &source[tokens[i].start..tokens[i].end];
        let mut index = Self {
            delimiter_closes: vec![None; tokens.len()],
            arm_ends: vec![None; tokens.len()],
            statement_ends: vec![None; tokens.len()],
            generic_angles: vec![false; tokens.len()],
        };
        let mut stack = Vec::new();
        for i in 0..tokens.len() {
            match text(i) {
                "(" | "{" | "[" | "do" => stack.push(i),
                ")" | "}" | "]" | "end" => {
                    if let Some(open) = stack.pop()
                        && matches!(
                            (text(open), text(i)),
                            ("(", ")") | ("{", "}") | ("[", "]") | ("do", "end")
                        )
                    {
                        index.delimiter_closes[open] = Some(i);
                    }
                }
                _ => {}
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
                    index.generic_angles[open] = true;
                    index.generic_angles[close] = true;
                    break;
                }
                if token.kind != "ident" && text(close) != "," {
                    break;
                }
            }
        }

        let mut statements = Vec::new();
        for region in analysis.regions() {
            if !matches!(region.kind.as_str(), "statement" | "match_arm") {
                continue;
            }
            let start = tokens.partition_point(|t| t.start < region.start);
            let end = tokens.partition_point(|t| t.start < region.end);
            let end = index.envelope_end(start, end);
            if region.kind == "statement" {
                statements.push((start, end));
            } else if start > 0
                && tokens.get(start).is_some_and(|t| t.start == region.start)
                && text(start - 1) == "|"
            {
                index.arm_ends[start - 1] = Some(end);
            }
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

    pub(crate) fn delimiter_close(&self, open: usize) -> Option<usize> {
        self.delimiter_closes[open]
    }

    pub(crate) fn arm_end(&self, bar: usize) -> Option<usize> {
        self.arm_ends[bar]
    }

    pub(crate) fn statement_end(&self, token: usize) -> Option<usize> {
        self.statement_ends[token]
    }

    pub(crate) fn generic_angle(&self, token: usize) -> bool {
        self.generic_angles[token]
    }
}
