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

const STRESS_SOURCE: &str = include_str!("../../tests/fixtures/stress/syntax.rv");

fn stress_options() -> impl Iterator<Item = FormatOptions> {
    [20, 24, 40, 80, 120, 240]
        .into_iter()
        .flat_map(|line_width| {
            [1, 2, 4, 8]
                .into_iter()
                .map(move |indent_width| FormatOptions {
                    line_width,
                    indent_width,
                })
        })
}

fn check_stress_source(name: &str, source: &str) -> usize {
    let mut cases = 0;
    for crlf in [false, true] {
        // Convert the complete input, including opaque literals and comments.
        let source = if crlf {
            source.replace('\n', "\r\n")
        } else {
            source.to_owned()
        };
        let analysis = crate::oracle::analyze(&source)
            .unwrap_or_else(|error| panic!("{name} crlf={crlf}: {error}"));
        for options in stress_options() {
            let output = format(&source, &options)
                .unwrap_or_else(|error| panic!("{name} {options:?} crlf={crlf}: {error}"));
            assert!(
                analysis.preserves(&output).unwrap(),
                "{name} {options:?} crlf={crlf}: syntax or token/comment tape changed"
            );
            assert_eq!(
                format(&output, &options).unwrap(),
                output,
                "{name} {options:?} crlf={crlf}: not idempotent"
            );
            cases += 1;
        }
    }
    cases
}

#[test]
fn match_file_read_preserves_guard_arrow_and_typed_patterns() {
    let source = include_str!("../../tests/fixtures/stress/match-file-read.rv");
    let expected = concat!(
        "let f = match fs.open(\"./readme.md\")?:read()\n",
        "  | {:ok, file} => file\n",
        "  | {:err, error} when error == :StatError => panic(\"file does not exist\")\n",
        "  | {error: string} => panic(\"#{error}!\")\n",
        "  | x => panic(\"unknown: \", x)\n",
    );
    let output = format(source, &FormatOptions::default()).unwrap();
    assert_eq!(output, expected);
    assert_preserved_and_idempotent(source, &output, &FormatOptions::default());
    let cases = check_stress_source("match fs.open()?:read()", source);
    eprintln!("file-read match: {cases} input/option combinations");
}

#[test]
fn synthetic_syntax_file_preserves_source_and_is_idempotent() {
    assert_eq!(
        format(STRESS_SOURCE, &FormatOptions::default()).unwrap(),
        include_str!("../../tests/fixtures/stress/syntax.expected.rv")
    );
    let cases = check_stress_source("complete syntax.rv", STRESS_SOURCE);
    eprintln!("complete stress file: {cases} input/option combinations");
}

#[test]
fn synthetic_syntax_sections_preserve_source_and_are_idempotent() {
    let mut cases = 0;
    let mut sections = 0;
    for section in STRESS_SOURCE.split("# case: ") {
        let name = section.lines().next().unwrap();
        // Include the marker as a comment when formatting an isolated section.
        let source = if sections == 0 {
            section.to_owned()
        } else {
            format!("# case: {section}")
        };
        cases += check_stress_source(name, &source);
        sections += 1;
    }
    eprintln!("{sections} stress sections: {cases} input/option combinations");
}

#[test]
fn composed_syntax_preserves_source_and_is_idempotent() {
    // Regressions in scope ownership, suffix fitting or generated pipe wrappers
    // can preserve isolated expressions but fail when expressions are nested.
    let expressions = [
        "()",
        "-(-value)^2",
        "not value and other or fallback",
        "value orelse fallback",
        "1 band 2 bor 3 bxor 4 shl 2 shr 1",
        "1..2..9",
        "\"left\"~string(value)",
        "\"שלום λ 文書 🙂\"",
        "\"#{value:?} #{other:p}\"",
        "'first\n  second'",
        "\"\"\"\n  first\n    second\n  \"\"\"",
        "`f(%value)`",
        "{[key]=value,nested={first,second}}",
        "values[1..2..8].field",
        "receiver:method(value):other()",
        "identity<T>(value)",
        "receiver.identity<T,U>(value,other)",
        "result()?",
        "fn(?x: int=1) -> int x",
        "fn(x) do consume(x);return x end",
        "if condition do yes() end else do no() end",
        "unless condition fallback",
        "match value |{:ok,x} when x>0=>x |_=>0",
        "match |condition=>1 |_=>0",
        "do let do=1;receiver.end end",
        "do/once break/once(value) end",
        "loop/again do break/again(value) end",
        "for i in 0..3 do consume(i) end",
        "for i in 0.. do break i end",
        "while condition do continue end",
        "value |> transform() |> fn(x) do consume(x);x end",
        "comp ({answer=6*7})",
        "spawn worker()",
        "import \"missing/module\"",
        "keep!(fn(x) x)",
        "1 ## opaque do end ## +2",
    ];
    let mut cases = 0;
    for expression in expressions {
        for source in [
            format!("let chosen=({expression});"),
            format!("do ({expression});following() end;"),
            format!("fn enclosing() do return ({expression}) end;"),
            format!("consume(({expression}),following_argument);"),
            format!("let chosen={{({expression}),following_value}};"),
            format!(
                "let piped_input=({expression});piped_input |> fn(result) do consume(result);result end;"
            ),
        ] {
            cases += check_stress_source(&source, &source);
        }
    }
    eprintln!("composed syntax: {cases} input/option combinations");
}

#[test]
fn invalid_stress_syntax_is_rejected_without_a_candidate() {
    // Near misses discovered while constructing the corpus are syntax errors
    // in the pinned frontend, rather than successful formatting examples.
    for source in [
        "result()!",
        "fn receiver.nested.field(x) x",
        "let value=(input: int)",
        "declare factory<T>=fn(x: T) -> T",
        "@native receiver.field=fn(x) x",
        "match value |1=>one, |2=>two",
        "match value |1 or 2=>one",
        "fn names(do,end) {do,end}",
        "@unknown fn f() 1",
        "let x:int=1",
        "type Broken={?name:string}",
        "proc missing(iter) {}",
        "let quote=`(unterminated`",
        "\"#{(unterminated}\"",
        "(do/once break/once(value) end) |> transform()",
    ] {
        for options in stress_options() {
            assert!(
                matches!(format(source, &options), Err(FormatError::Syntax { .. })),
                "{source:?} {options:?}"
            );
        }
    }
}
