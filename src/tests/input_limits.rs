use crate::{FormatError, FormatOptions, MAX_SOURCE_BYTES, format};

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
