use super::assert_preserved_and_idempotent;
use crate::{FormatError, FormatOptions, format};

struct Fixture {
    name: &'static str,
    source: &'static str,
}

const VALID: &[Fixture] = &[
    Fixture {
        name: "demo.rv",
        source: include_str!("../../tests/fixtures/upstream/demo.rv"),
    },
    Fixture {
        name: "pipes.rv",
        source: include_str!("../../tests/fixtures/upstream/pipes.rv"),
    },
    Fixture {
        name: "proc.rv",
        source: include_str!("../../tests/fixtures/upstream/proc.rv"),
    },
    Fixture {
        name: "types.rv",
        source: include_str!("../../tests/fixtures/upstream/types.rv"),
    },
    Fixture {
        name: "errors.rv",
        source: include_str!("../../tests/fixtures/upstream/errors.rv"),
    },
    Fixture {
        name: "misc-docs.rv",
        source: include_str!("../../tests/fixtures/upstream/misc-docs.rv"),
    },
    Fixture {
        name: "docs-annotations.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-annotations.rv"),
    },
    Fixture {
        name: "docs-type-alias.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-type-alias.rv"),
    },
    Fixture {
        name: "docs-ambient.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-ambient.rv"),
    },
    Fixture {
        name: "docs-if.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-if.rv"),
    },
    Fixture {
        name: "docs-loops.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-loops.rv"),
    },
    Fixture {
        name: "docs-labels.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-labels.rv"),
    },
    Fixture {
        name: "docs-ranges.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-ranges.rv"),
    },
    Fixture {
        name: "docs-match.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-match.rv"),
    },
    Fixture {
        name: "docs-match-comma.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-match-comma.rv"),
    },
    Fixture {
        name: "docs-doc-comments.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-doc-comments.rv"),
    },
    Fixture {
        name: "docs-comptime.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-comptime.rv"),
    },
    Fixture {
        name: "docs-proc.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-proc.rv"),
    },
    Fixture {
        name: "docs-quotes.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-quotes.rv"),
    },
    Fixture {
        name: "docs-proc-quote.rv",
        source: include_str!("../../tests/fixtures/upstream/docs-proc-quote.rv"),
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

#[test]
fn corpus_preserves_source_and_is_idempotent_with_reviewed_outputs() {
    let mut cases = 0;
    let mut goldens = 0;
    for fixture in VALID {
        for options in combinations() {
            let formatted = output(fixture, &options);
            assert_preserved_and_idempotent(fixture.source, &formatted, &options);
            if let Some(expected) = expected_output(fixture.name, &options) {
                assert_ne!(
                    formatted, fixture.source,
                    "{} {options:?} must actually format",
                    fixture.name
                );
                assert_eq!(formatted, expected, "{} {options:?}", fixture.name);
                goldens += 1;
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 120);
    assert_eq!(goldens, 4);
}

fn expected_output(name: &str, options: &FormatOptions) -> Option<&'static str> {
    match (name, options.line_width, options.indent_width) {
        ("docs-proc.rv", 24, 2) => Some(include_str!(
            "../../tests/fixtures/upstream/expected/docs-proc-24-2.rv"
        )),
        ("docs-proc.rv", 120, 4) => Some(include_str!(
            "../../tests/fixtures/upstream/expected/docs-proc-120-4.rv"
        )),
        ("docs-match.rv", 80, 4) => Some(include_str!(
            "../../tests/fixtures/upstream/expected/docs-match-80-4.rv"
        )),
        ("docs-ambient.rv", 24, 4) => Some(include_str!(
            "../../tests/fixtures/upstream/expected/docs-ambient-24-4.rv"
        )),
        _ => None,
    }
}

#[test]
fn documented_rejected_fixture_is_a_syntax_error() {
    let source = include_str!("../../tests/fixtures/upstream/rejected/docs-type-name.rv");
    for options in combinations() {
        assert!(
            matches!(format(source, &options), Err(FormatError::Syntax { .. })),
            "rejected/docs-type-name.rv {options:?}"
        );
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
