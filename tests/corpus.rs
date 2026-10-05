use revofmt::{FormatError, FormatOptions, format};

// Match tests/formatting.rs: compile the crate-private ABI oracle here without
// expanding the public API. Its nine unit tests also run in this test binary.
#[path = "../src/oracle.rs"]
mod oracle;

struct Fixture {
    name: &'static str,
    source: &'static str,
}

const VALID: &[Fixture] = &[
    Fixture {
        name: "demo.rv",
        source: include_str!("fixtures/upstream/demo.rv"),
    },
    Fixture {
        name: "pipes.rv",
        source: include_str!("fixtures/upstream/pipes.rv"),
    },
    Fixture {
        name: "proc.rv",
        source: include_str!("fixtures/upstream/proc.rv"),
    },
    Fixture {
        name: "types.rv",
        source: include_str!("fixtures/upstream/types.rv"),
    },
    Fixture {
        name: "errors.rv",
        source: include_str!("fixtures/upstream/errors.rv"),
    },
    Fixture {
        name: "misc-docs.rv",
        source: include_str!("fixtures/upstream/misc-docs.rv"),
    },
    Fixture {
        name: "docs-annotations.rv",
        source: include_str!("fixtures/upstream/docs-annotations.rv"),
    },
    Fixture {
        name: "docs-type-alias.rv",
        source: include_str!("fixtures/upstream/docs-type-alias.rv"),
    },
    Fixture {
        name: "docs-ambient.rv",
        source: include_str!("fixtures/upstream/docs-ambient.rv"),
    },
    Fixture {
        name: "docs-if.rv",
        source: include_str!("fixtures/upstream/docs-if.rv"),
    },
    Fixture {
        name: "docs-loops.rv",
        source: include_str!("fixtures/upstream/docs-loops.rv"),
    },
    Fixture {
        name: "docs-labels.rv",
        source: include_str!("fixtures/upstream/docs-labels.rv"),
    },
    Fixture {
        name: "docs-ranges.rv",
        source: include_str!("fixtures/upstream/docs-ranges.rv"),
    },
    Fixture {
        name: "docs-match.rv",
        source: include_str!("fixtures/upstream/docs-match.rv"),
    },
    Fixture {
        name: "docs-match-comma.rv",
        source: include_str!("fixtures/upstream/docs-match-comma.rv"),
    },
    Fixture {
        name: "docs-doc-comments.rv",
        source: include_str!("fixtures/upstream/docs-doc-comments.rv"),
    },
    Fixture {
        name: "docs-comptime.rv",
        source: include_str!("fixtures/upstream/docs-comptime.rv"),
    },
    Fixture {
        name: "docs-proc.rv",
        source: include_str!("fixtures/upstream/docs-proc.rv"),
    },
    Fixture {
        name: "docs-quotes.rv",
        source: include_str!("fixtures/upstream/docs-quotes.rv"),
    },
    Fixture {
        name: "docs-proc-quote.rv",
        source: include_str!("fixtures/upstream/docs-proc-quote.rv"),
    },
];

fn combinations() -> impl Iterator<Item = FormatOptions> {
    [24, 80, 120].into_iter().flat_map(|line_width| {
        [2, 4].into_iter().map(move |indent_width| FormatOptions {
            line_width,
            indent_width,
        })
    })
}

fn output(fixture: &Fixture, options: &FormatOptions) -> String {
    format(fixture.source, options)
        .unwrap_or_else(|error| panic!("{} {options:?}: {error}", fixture.name))
}

fn tape<'a>(source: &'a str, analysis: &'a oracle::Analysis) -> Vec<(&'a str, &'a str)> {
    analysis
        .tokens
        .iter()
        .map(|token| (token.kind.as_str(), &source[token.start..token.end]))
        .collect()
}

#[test]
fn corpus_reparses_and_preserves_raw_interleaved_tape() {
    for fixture in VALID {
        let before = oracle::analyze(fixture.source)
            .unwrap_or_else(|error| panic!("{} original: {error}", fixture.name));
        for options in combinations() {
            let formatted = output(fixture, &options);
            let after = oracle::analyze(&formatted)
                .unwrap_or_else(|error| panic!("{} {options:?} reparse: {error}", fixture.name));
            assert_eq!(
                tape(fixture.source, &before),
                tape(&formatted, &after),
                "{} {options:?}",
                fixture.name
            );
        }
    }
}

#[test]
fn corpus_preserves_complete_ast_modulo_coordinates() {
    for fixture in VALID {
        for options in combinations() {
            let formatted = output(fixture, &options);
            assert!(
                oracle::equivalent(fixture.source, &formatted)
                    .unwrap_or_else(|error| panic!("{} {options:?} AST: {error}", fixture.name)),
                "{} {options:?}",
                fixture.name
            );
        }
    }
}

#[test]
fn corpus_is_idempotent() {
    for fixture in VALID {
        for options in combinations() {
            let formatted = output(fixture, &options);
            let second = format(&formatted, &options).unwrap_or_else(|error| {
                panic!("{} {options:?} second pass: {error}", fixture.name)
            });
            assert_eq!(second, formatted, "{} {options:?}", fixture.name);
        }
    }
}

#[test]
fn documented_rejected_fixture_is_a_syntax_error() {
    let source = include_str!("fixtures/upstream/rejected/docs-type-name.rv");
    for options in combinations() {
        assert!(
            matches!(format(source, &options), Err(FormatError::Syntax { .. })),
            "rejected/docs-type-name.rv {options:?}"
        );
    }
}

#[test]
fn reviewed_expected_outputs_show_reflow_and_indentation() {
    for (name, width, indent, expected) in [
        (
            "docs-proc.rv",
            24,
            2,
            include_str!("fixtures/upstream/expected/docs-proc-24-2.rv"),
        ),
        (
            "docs-proc.rv",
            120,
            4,
            include_str!("fixtures/upstream/expected/docs-proc-120-4.rv"),
        ),
        (
            "docs-match.rv",
            80,
            4,
            include_str!("fixtures/upstream/expected/docs-match-80-4.rv"),
        ),
        (
            "docs-ambient.rv",
            24,
            4,
            include_str!("fixtures/upstream/expected/docs-ambient-24-4.rv"),
        ),
    ] {
        let fixture = VALID.iter().find(|fixture| fixture.name == name).unwrap();
        let options = FormatOptions {
            line_width: width,
            indent_width: indent,
        };
        let formatted = output(fixture, &options);
        assert_ne!(
            formatted, fixture.source,
            "{name} {options:?} must actually format"
        );
        assert_eq!(formatted, expected, "{name} {options:?}");
    }
}

#[test]
fn whitespace_traps_keep_raw_tape_but_change_ast() {
    // Both sides parse, and every non-whitespace byte is unchanged. Only full
    // AST comparison detects these unsafe changes to call/statement boundaries.
    for (name, source, candidate) in [
        ("call adjacency", "f(1)", "f (1)"),
        ("newline before string argument", "f 'hello'", "f\n'hello'"),
    ] {
        let before = oracle::analyze(source).unwrap();
        let after = oracle::analyze(candidate).unwrap();
        assert_eq!(tape(source, &before), tape(candidate, &after), "{name}");
        assert!(!oracle::equivalent(source, candidate).unwrap(), "{name}");
    }
}

#[test]
fn moved_comments_and_respelled_literals_require_raw_tape_validation() {
    // The AST permits these edits. The interleaved tape must reject them.
    for (name, source, candidate) in [
        (
            "moved block comment",
            "let x = 1 ## note ##",
            "## note ## let x = 1",
        ),
        ("respelled string", "let x = 'same'", "let x = \"same\""),
    ] {
        let before = oracle::analyze(source).unwrap();
        let after = oracle::analyze(candidate).unwrap();
        assert!(oracle::equivalent(source, candidate).unwrap(), "{name}");
        assert_ne!(tape(source, &before), tape(candidate, &after), "{name}");
    }
}

#[test]
fn malformed_sources_never_produce_candidates() {
    for source in [
        "let x =",
        "\"unterminated",
        "let x = 1 )",
        "proc () 1",
        "$",
        "let x=1\n#! late !#",
    ] {
        for options in combinations() {
            assert!(
                matches!(format(source, &options), Err(FormatError::Syntax { .. })),
                "{source:?} {options:?}"
            );
        }
    }
}
