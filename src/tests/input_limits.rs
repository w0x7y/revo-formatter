use super::assert_preserved_and_idempotent;
use crate::{FormatError, FormatOptions, MAX_SOURCE_BYTES, format};

#[test]
fn analysis_admits_source_before_parsing() {
    for source in [
        format!("'{}'", "x".repeat(MAX_SOURCE_BYTES)),
        "let x=1;".repeat(1200),
    ] {
        assert!(matches!(
            crate::oracle::analyze(&source),
            Err(FormatError::Validation(message)) if message.contains("input complexity limit")
        ));
    }
    assert!(crate::oracle::analyze("let x=1").is_ok());
}

fn blocks(depth: usize, body: &str) -> String {
    format!("{}{body}{}", "do\n".repeat(depth), "\nend".repeat(depth))
}

#[test]
fn ordinary_blocks_share_admission_with_pipes_and_decoded_bodies() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let options = FormatOptions::default();
            let accepted = [
                blocks(200, "foo.end"),
                format!("{}1", "1 |> ".repeat(112)),
                blocks(200, &format!("{}1", "1 |> ".repeat(87))),
                blocks(199, "1+1+1+1+1"),
                blocks(100, &std::iter::repeat_n("1", 401).collect::<Vec<_>>().join("+")),
                blocks(150, &format!("{}{}1{}", "(".repeat(16), "fn() ".repeat(66), ")".repeat(16))),
                blocks(198, &format!("\"#{{{}}}\"", blocks(1, "1"))),
                blocks(198, &format!("`{}`", blocks(1, "1"))),
                blocks(197, &format!("\"outer #{{\\\"inner #{{{}}}\\\":p}}\"", blocks(1, "1"))),
                format!("\"#{{{}}} #{{{}}}\"", blocks(99, "1"), blocks(100, "1")),
                format!("'{}' ##{}##", "do end ".repeat(400), "do end ".repeat(400)),
                format!("a{}", ".do".repeat(200)),
                blocks(64, &format!("{}1", "not ".repeat(272))),
                format!("\"#{{{}}} #{{{}}}\"", blocks(100, "1"), format_args!("{}1", "not ".repeat(198))),
                format!("\"#{{{}}} #{{{}}}\"", format_args!("{}1", "not ".repeat(198)), blocks(100, "1")),
            ];
            for (index, source) in accepted.into_iter().enumerate() {
                let output = format(&source, &options)
                    .unwrap_or_else(|error| panic!("accepted block case {index}: {error}"));
                assert_preserved_and_idempotent(&source, &output, &options);
            }
            // Sample both sides of the shared parser/layout/tree boundary, with
            // functions inside blocks and blocks inside functions.
            for (depth, functions) in [(0, 307), (20, 303), (64, 270), (100, 198), (150, 98), (190, 18), (198, 2)] {
                let prefixes = "fn() ".repeat(functions);
                for source in [blocks(depth, &format!("{prefixes}foo.end")), format!("{prefixes}{}", blocks(depth, "foo.end"))] {
                    let output = format(&source, &options)
                        .unwrap_or_else(|error| panic!("blocks {depth}, functions {functions}: {error}"));
                    assert_preserved_and_idempotent(&source, &output, &options);
                }
            }
            for (depth, unary) in [(154, 182), (155, 180), (195, 20)] {
                let prefixes = "- ".repeat(unary);
                for source in [blocks(depth, &format!("{prefixes}1")), format!("{prefixes}{}", blocks(depth, "1"))] {
                    let output = format(&source, &options).unwrap();
                    assert_preserved_and_idempotent(&source, &output, &options);
                }
            }
            for prefix in ["fn() do\n", "proc p!(x) do\n", "if x do\n", "unless x do\n", "loop do\n", "for x in xs do\n", "while x do\n"] {
                let source = format!("{}1{}", prefix.repeat(132), "\nend".repeat(132));
                let output = format(&source, &options).unwrap();
                assert_preserved_and_idempotent(&source, &output, &options);
            }
            for prefix in ["fn() proc p!(x) do\n", "proc p!(x) fn() do\n"] {
                let source = format!("{}1{}", prefix.repeat(99), "\nend".repeat(99));
                let output = format(&source, &options).unwrap();
                assert_preserved_and_idempotent(&source, &output, &options);
            }
            let rejected = [
                blocks(201, "1"),
                blocks(300, "foo.end"),
                blocks(336, "foo.end"),
                blocks(199, &format!("{}1", "fn() ".repeat(135))),
                blocks(100, &format!("{}1", "fn() ".repeat(199))),
                format!("{}{}", "fn() ".repeat(199), blocks(100, "1")),
                blocks(100, &format!("{}1", "not ".repeat(201))),
                format!("{}{}", "not ".repeat(201), blocks(100, "1")),
                blocks(200, &format!("{}1", "1 |> ".repeat(88))),
                blocks(199, "1+1+1+1+1+1"),
                format!("\"#{{{}}} #{{{}}}\"", blocks(100, "1"), blocks(100, "1")),
                format!("`{}; {}`", blocks(100, "1"), blocks(100, "1")),
                blocks(198, &format!("\"outer #{{\\\"inner #{{{}}}\\\":p}}\"", blocks(1, "1"))),
                format!("a{}", ".do".repeat(201)),
                format!("\"#{{{}}} #{{{}}}\"", blocks(100, "1"), format_args!("{}1", "not ".repeat(199))),
                format!("\"#{{{}}} #{{{}}}\"", format_args!("{}1", "not ".repeat(199)), blocks(100, "1")),
            ];
            for (index, source) in rejected.into_iter().enumerate() {
                assert!(matches!(
                    format(&source, &options),
                    Err(FormatError::Validation(message)) if message.contains("input complexity limit")
                ), "rejected block case {index}");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn admission_precedes_recursive_work_on_a_two_mib_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let chain = std::iter::repeat_n("1", 800).collect::<Vec<_>>().join("+");
            let moderate_chain = std::iter::repeat_n("1", 700).collect::<Vec<_>>().join("+");
            let match_chain = std::iter::repeat_n("1", 600).collect::<Vec<_>>().join("+");
            for source in [
                chain.clone(),
                format!("{}{chain}", "not ".repeat(100)),
                format!("{}{moderate_chain}", "fn() ".repeat(98)),
                format!("{}{}{}", "(".repeat(20), moderate_chain, ")".repeat(20)),
                format!("{}{match_chain}", "match x | _ => ".repeat(20)),
                format!("{}1", "fn() ".repeat(300)),
                format!("{}a{}", "fn() ".repeat(100), ".f".repeat(690)),
                format!("{}a{}", "not ".repeat(20), ":f()".repeat(430)),
                format!("a{}", ".f".repeat(899)),
                format!("a{}", ":f()".repeat(448)),
                std::iter::repeat_n("1", 800).collect::<Vec<_>>().join("%"),
                std::iter::repeat_n("1", 800).collect::<Vec<_>>().join(">"),
                format!("{}{}", "not ".repeat(100), std::iter::repeat_n("1", 800).collect::<Vec<_>>().join("%")),
                format!("{}a{}", "fn() ".repeat(50), ":f()".repeat(398)),
                format!("'{}'", "x".repeat(100_000)),
                "let q = `f(%x)`; let s = \"outer #{\\\"inner #{x}\\\":p}\"".into(),
            ].into_iter().enumerate() {
                let (index, source) = source;
                let output = format(&source, &FormatOptions::default()).unwrap_or_else(|error| panic!("accepted case {index}: {error}"));
                assert_eq!(format(&output, &FormatOptions::default()).unwrap(), output);
            }
            let deep = format!("{}1{}", "(".repeat(3000), ")".repeat(3000));
            for source in [
                deep.clone(),
                format!("{}1", "(".repeat(3000)),
                format!("{}1", "not ".repeat(3000)),
                std::iter::repeat_n("1", 6000).collect::<Vec<_>>().join("+"),
                std::iter::repeat_n("1", 1500).collect::<Vec<_>>().join("%"),
                std::iter::repeat_n("1", 1500).collect::<Vec<_>>().join(">"),
                format!("a{}", ".field".repeat(4000)),
                format!("a{}", ".f".repeat(1500)),
                format!("a{}", ":f()".repeat(760)),
                format!("\"#{{{deep}}}\""),
                format!("`{deep}`"),
                format!("{}1", "fn() ".repeat(500)),
                format!("declare X = {}number", "!".repeat(1500)),
                format!("{}1", "match x | _ => ".repeat(330)),
                format!("{}{}", "match x | _ => ".repeat(330), chain),
            ] {
                assert!(matches!(
                    format(&source, &FormatOptions::default()),
                    Err(FormatError::Validation(message)) if message.contains("input complexity limit")
                ));
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn source_limit_covers_candidates_and_retains_error_categories() {
    let options = FormatOptions::default();
    let stable = format!("'{}'\n", "x".repeat(MAX_SOURCE_BYTES - 3));
    assert_eq!(stable.len(), MAX_SOURCE_BYTES);
    assert_eq!(format(&stable, &options).unwrap(), stable);
    let missing_newline = &stable[..stable.len() - 1];
    assert_eq!(format(missing_newline, &options).unwrap(), stable);
    let exact_sized_literal = format!("'{}'", "x".repeat(MAX_SOURCE_BYTES - 2));
    assert!(
        matches!(format(&exact_sized_literal, &options), Err(FormatError::Validation(message)) if message.contains("source bytes"))
    );
    let oversized = "x".repeat(MAX_SOURCE_BYTES + 1);
    assert!(
        matches!(format(&oversized, &options), Err(FormatError::Validation(message)) if message.contains("source bytes"))
    );
    assert!(matches!(
        format(
            &oversized,
            &FormatOptions {
                indent_width: 0,
                ..options
            }
        ),
        Err(FormatError::InvalidOptions(_))
    ));
    for source in ["let x =", "\"unterminated", "\"#{let x =}\""] {
        assert!(matches!(
            format(source, &options),
            Err(FormatError::Syntax { .. })
        ));
    }
}
