use revofmt::{FormatError, FormatOptions, format};

#[path = "../src/oracle.rs"]
mod oracle;

fn check(source: &str, expected: &str, options: FormatOptions) {
    let output = format(source, &options).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    assert_eq!(output, expected, "source: {source:?}");
    assert!(oracle::equivalent(source, &output).unwrap());
    let before = oracle::analyze(source).unwrap();
    let after = oracle::analyze(&output).unwrap();
    let tape = |s: &str, a: &oracle::Analysis| {
        a.tokens
            .iter()
            .map(|t| (t.kind.clone(), s[t.start..t.end].to_owned()))
            .collect::<Vec<_>>()
    };
    assert_eq!(tape(source, &before), tape(&output, &after));
    assert_eq!(format(&output, &options).unwrap(), output);
}

#[test]
fn assignment_and_blocks() {
    check(
        "let x=1+2*3",
        "let x = 1 + 2 * 3\n",
        FormatOptions::default(),
    );
    check(
        "fn f() do\nlet x=1\nx\nend",
        "fn f() do\n  let x = 1\n  x\nend\n",
        FormatOptions::default(),
    );
    check(
        "fn f() do\ndo\nx\nend\nend",
        "fn f() do\n    do\n        x\n    end\nend\n",
        FormatOptions {
            indent_width: 4,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn width_driven_calls_and_tables() {
    let narrow = FormatOptions {
        line_width: 24,
        ..FormatOptions::default()
    };
    check(
        "print(first_argument, second_argument)",
        "print(\n  first_argument,\n  second_argument\n)\n",
        narrow,
    );
    check(
        "print(first_argument, second_argument)",
        "print(first_argument, second_argument)\n",
        FormatOptions::default(),
    );
    check(
        "let t={first_argument,second_argument}",
        "let t = {\n  first_argument,\n  second_argument\n}\n",
        narrow,
    );
}

#[test]
fn expression_continuations_and_match_bodies() {
    check(
        "let x=first_argument+second_argument",
        "let x = first_argument +\n  second_argument\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
    check(
        "match x\n| 1 =>\nf(1)\n| _ =>\nf(2)",
        "match x\n  | 1 =>\n    f(1)\n  | _ =>\n    f(2)\n",
        FormatOptions::default(),
    );
}

#[test]
fn signatures_adjacency_and_ranges() {
    let defaults = FormatOptions::default();
    for (source, expected) in [
        ("f (1)", "f (1)\n"),
        ("f<T>(x)", "f<T>(x)\n"),
        ("f <T>(x)", "f <T>(x)\n"),
        ("fn id<T>(x: T)->T x", "fn id<T>(x: T) -> T x\n"),
        (
            "fn obj:method(?x: number=1) x",
            "fn obj:method(?x: number = 1) x\n",
        ),
        ("fn obj.field(x) x", "fn obj.field(x) x\n"),
        (
            "for i in 0.. do\nprint(i)\nend",
            "for i in 0.. do\n  print(i)\nend\n",
        ),
        ("for i in 0..2..10 do i end", "for i in 0..2..10 do i end\n"),
        (
            "loop/outer do break/outer nil end",
            "loop/outer do break/outer nil end\n",
        ),
        ("value ?", "value ?\n"),
        ("let x=a<2 and b>3", "let x = a < 2 and b > 3\n"),
        ("check!(x)", "check!(x)\n"),
        ("let y=xs |> fn(x) x+1", "let y = xs |> fn(x) x + 1\n"),
        ("f ## comment ##(1)", "f ## comment ##(1)\n"),
        ("f ## comment ## (1)", "f ## comment ## (1)\n"),
        ("f \"hello\"", "f \"hello\"\n"),
    ] {
        check(source, expected, defaults);
    }
}

#[test]
fn opaque_and_record_doc_fixtures() {
    for (source, expected) in [
        (
            include_str!("fixtures/formatting/opaque.rv"),
            include_str!("fixtures/formatting/opaque.expected.rv"),
        ),
        (
            include_str!("fixtures/formatting/record-docs.rv"),
            include_str!("fixtures/formatting/record-docs.expected.rv"),
        ),
    ] {
        check(source, expected, FormatOptions::default());
    }
}

#[test]
fn line_endings_blank_lines_and_empty_source() {
    for (source, expected) in [
        ("", ""),
        (" \t\n ", "\n"),
        (" \r\n ", "\r\n"),
        ("let x=1\n\n\n\nlet y=2\n\n", "let x = 1\n\nlet y = 2\n"),
        (
            "do\r\n# hi\r\nlet x=1\r\nend\r\n",
            "do\r\n  # hi\r\n  let x = 1\r\nend\r\n",
        ),
        ("# trailing comment\r\n", "# trailing comment\r\n"),
        (
            "let x='first\nsecond'\r\n# end\r\n",
            "let x = 'first\nsecond'\r\n# end\r\n",
        ),
        ("let x=1 # attached\nx", "let x = 1 # attached\nx\n"),
    ] {
        check(source, expected, FormatOptions::default());
    }
}

#[test]
fn validates_options_before_syntax_and_reports_malformed_input() {
    for options in [
        FormatOptions {
            indent_width: 0,
            line_width: 80,
        },
        FormatOptions {
            indent_width: 9,
            line_width: 80,
        },
        FormatOptions {
            indent_width: 2,
            line_width: 19,
        },
        FormatOptions {
            indent_width: 2,
            line_width: 241,
        },
    ] {
        assert!(matches!(
            format("$", &options),
            Err(FormatError::InvalidOptions(_))
        ));
    }
    for source in [
        "let x =",
        "\"unterminated",
        "let x=1 )",
        "proc () 1",
        "$",
        "let x=1\n#! late !#",
        "value !",
    ] {
        assert!(
            matches!(format(source, &FormatOptions::default()), Err(FormatError::Syntax { message, offset }) if !message.is_empty() && offset <= source.len()),
            "{source}"
        );
    }
    for (indent_width, line_width) in [(1, 20), (8, 240)] {
        let expected = format!("do\n{}x\nend\n", " ".repeat(indent_width));
        check(
            "do\nx\nend",
            &expected,
            FormatOptions {
                indent_width,
                line_width,
            },
        );
    }
}

#[test]
fn unicode_display_columns_and_long_opaque_tokens() {
    let narrow = FormatOptions {
        line_width: 20,
        ..FormatOptions::default()
    };
    check(
        "f(\"日本語日本語日本語\")",
        "f(\n  \"日本語日本語日本語\"\n)\n",
        narrow,
    );
    check("f(\"éééééé\")", "f(\"éééééé\")\n", narrow);
    check(
        "let x='this literal is longer than twenty columns'",
        "let x = 'this literal is longer than twenty columns'\n",
        narrow,
    );
}

#[test]
fn procedural_macros_preserve_syntax_modulo_coordinates() {
    // Parsing is sufficient: never execute this coordinate-inspecting macro.
    let source = "proc offset!(iter) do let f=iter:next(); {{:number, f[1][0][1][0]}} end; offset!(fn(  x) x)";
    let output = format(source, &FormatOptions::default()).unwrap();
    assert!(output.contains("fn(x) x"));
    check(source, &output, FormatOptions::default());
}

#[test]
fn nested_interpolation_escaped_templates_and_opaque_line_endings() {
    for (source, expected) in [
        (
            r##"let x="#{fn() \"nested\"}""##,
            r##"let x = "#{fn() \"nested\"}""##.to_owned() + "\n",
        ),
        (r#"let x=`"a\`b"`"#, r#"let x = `"a\`b"`"#.to_owned() + "\n"),
        (
            "let x='first\r\nsecond'\nlet y=1\n",
            "let x = 'first\r\nsecond'\nlet y = 1\n".to_owned(),
        ),
        (
            "#!/usr/bin/env revo\nmodule prose\n!#\nlet x=1",
            "#!/usr/bin/env revo\nmodule prose\n!#\nlet x = 1\n".to_owned(),
        ),
    ] {
        check(source, &expected, FormatOptions::default());
    }
}

#[test]
fn binary_groups_fit_independently_of_other_statements() {
    check(
        "let x=1+2*3\nlet y=4",
        "let x = 1 + 2 * 3\nlet y = 4\n",
        FormatOptions::default(),
    );
    check(
        "fn f() do\nlet x=1+2\nx\nend",
        "fn f() do\n  let x = 1 + 2\n  x\nend\n",
        FormatOptions::default(),
    );
    let narrow = FormatOptions {
        line_width: 24,
        ..FormatOptions::default()
    };
    check(
        "let x=first_argument+second_argument\nlet y=1+2",
        "let x = first_argument +\n  second_argument\nlet y = 1 + 2\n",
        narrow,
    );
    check(
        "fn f() do\nlet x=first_value+second_value\nlet y=1+2\ny\nend",
        "fn f() do\n  let x = first_value +\n    second_value\n  let y = 1 + 2\n  y\nend\n",
        narrow,
    );
}

#[test]
fn match_arm_delimiter_envelopes_compose_indentation() {
    for (source, expected_two, expected_four) in [
        (
            "match x\n| _ => do\nf()\nend",
            "match x\n  | _ => do\n    f()\n  end\n",
            "match x\n    | _ => do\n        f()\n    end\n",
        ),
        (
            "match x\n| _ => do\ndo\nf()\nend\nend",
            "match x\n  | _ => do\n    do\n      f()\n    end\n  end\n",
            "match x\n    | _ => do\n        do\n            f()\n        end\n    end\n",
        ),
        (
            "match x\n| _ => {\nx=1,\ny=2\n}",
            "match x\n  | _ => {\n    x = 1,\n    y = 2\n  }\n",
            "match x\n    | _ => {\n        x = 1,\n        y = 2\n    }\n",
        ),
        (
            "match x\n| 1 => do\nf()\nend\n| _ => {\nx=1\n}",
            "match x\n  | 1 => do\n    f()\n  end\n  | _ => {\n    x = 1\n  }\n",
            "match x\n    | 1 => do\n        f()\n    end\n    | _ => {\n        x = 1\n    }\n",
        ),
        (
            "do\nmatch x\n| _ => do\nf()\nend\nend",
            "do\n  match x\n    | _ => do\n      f()\n    end\nend\n",
            "do\n    match x\n        | _ => do\n            f()\n        end\nend\n",
        ),
        (
            "match x\n| _ =>\ndo\nf()\nend",
            "match x\n  | _ =>\n    do\n      f()\n    end\n",
            "match x\n    | _ =>\n        do\n            f()\n        end\n",
        ),
        (
            "match x\n| _ => # body\ndo\nf()\nend",
            "match x\n  | _ => # body\n    do\n      f()\n    end\n",
            "match x\n    | _ => # body\n        do\n            f()\n        end\n",
        ),
    ] {
        check(
            source,
            expected_two,
            FormatOptions {
                indent_width: 2,
                line_width: 24,
            },
        );
        check(
            source,
            expected_four,
            FormatOptions {
                indent_width: 4,
                line_width: 80,
            },
        );
    }
    check(
        "match x\n| _ => {first_argument,second_argument}",
        "match x\n  | _ => {\n    first_argument,\n    second_argument\n  }\n",
        FormatOptions {
            indent_width: 2,
            line_width: 24,
        },
    );
    check(
        "match x\n| _ => {first_argument,second_argument}",
        "match x\n    | _ => {first_argument, second_argument}\n",
        FormatOptions {
            indent_width: 4,
            line_width: 80,
        },
    );
}

#[test]
fn call_arm_delimiters_compose_with_reflow() {
    check(
        "match x\n| _ => f(first_argument,second_argument)",
        "match x\n  | _ => f(\n    first_argument,\n    second_argument\n  )\n",
        FormatOptions {
            indent_width: 2,
            line_width: 24,
        },
    );
    check(
        "match x\n| _ =>\nf(first_argument,second_argument)",
        "match x\n    | _ =>\n        f(\n            first_argument,\n            second_argument\n        )\n",
        FormatOptions {
            indent_width: 4,
            line_width: 24,
        },
    );
}

#[test]
fn statement_rhs_envelopes_compose_indentation() {
    for (source, expected) in [
        (
            "let x=first_argument+do\nf()\nend\nlet y=2",
            "let x = first_argument +\n  do\n    f()\n  end\nlet y = 2\n",
        ),
        (
            "let x=first_argument+do\ndo\nf()\nend\nend\nlet y=2",
            "let x = first_argument +\n  do\n    do\n      f()\n    end\n  end\nlet y = 2\n",
        ),
        (
            "let x=first_argument+{\na=1,\nb=2\n}\nlet y=2",
            "let x = first_argument +\n  {\n    a = 1,\n    b = 2\n  }\nlet y = 2\n",
        ),
        (
            "let x=first_argument+(\nsecond_argument\n)\nlet y=2",
            "let x = first_argument +\n  (\n    second_argument\n  )\nlet y = 2\n",
        ),
        (
            "do\nlet x=first_argument+(\nsecond_argument\n) # rhs\nlet y=2\nend\nlet z=3",
            "do\n  let x = first_argument +\n    (\n      second_argument\n    ) # rhs\n  let y = 2\nend\nlet z = 3\n",
        ),
        (
            "do\nmatch x\n| 1 => first_argument+do\nf()\nend\n| _ => second_argument+(\ng()\n)\nend\nlet y=2",
            "do\n  match x\n    | 1 => first_argument +\n      do\n        f()\n      end\n    | _ => second_argument +\n      (\n        g()\n      )\nend\nlet y = 2\n",
        ),
    ] {
        for indent_width in [2, 4] {
            let expected = expected
                .lines()
                .map(|line| {
                    let spaces = line.len() - line.trim_start().len();
                    format!(
                        "{}{}\n",
                        " ".repeat(spaces / 2 * indent_width),
                        line.trim_start()
                    )
                })
                .collect::<String>();
            for line_width in [24, 80] {
                check(
                    source,
                    &expected,
                    FormatOptions {
                        indent_width,
                        line_width,
                    },
                );
            }
        }
    }
}

#[test]
fn flat_binary_chain_shares_continuation_indent() {
    let options = FormatOptions {
        line_width: 24,
        ..FormatOptions::default()
    };
    let source = format!(
        "let x={}",
        std::iter::repeat_n("value", 8)
            .collect::<Vec<_>>()
            .join("+")
    );
    let output = format(&source, &options).unwrap();
    assert!(output.lines().count() > 1);
    assert!(
        output
            .lines()
            .skip(1)
            .all(|line| line.bytes().take_while(|b| *b == b' ').count() == 2),
        "{output}"
    );
    check(
        &source,
        "let x = value +\n  value +\n  value +\n  value +\n  value +\n  value + value + value\n",
        options,
    );
}

#[test]
fn flat_binary_chain_output_grows_linearly() {
    let options = FormatOptions {
        line_width: 20,
        ..FormatOptions::default()
    };
    let source = format!(
        "let x={}",
        std::iter::repeat_n("1", 800).collect::<Vec<_>>().join("+")
    );
    let output = format(&source, &options).unwrap();
    assert!(
        output.len() <= source.len() * 8,
        "{} output bytes for {} source bytes",
        output.len(),
        source.len()
    );
    check(&source, &output, options);
}

#[test]
fn binary_continuations_end_before_adjacent_list_items() {
    check(
        "f(first_argument+second_argument,third_argument+fourth_argument)",
        "f(\n  first_argument +\n    second_argument,\n  third_argument +\n    fourth_argument\n)\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn binary_segments_compose_with_nested_scopes_and_hard_breaks() {
    for (source, expected) in [
        (
            "let x=first_argument+second_argument*third_argument-fourth_argument/last_argument\nlet y=1+2",
            "let x = first_argument +\n  second_argument *\n  third_argument -\n  fourth_argument /\n  last_argument\nlet y = 1 + 2\n",
        ),
        (
            "let x=first_argument+(second_argument+third_argument)+last_argument\nlet y=2",
            "let x = first_argument +\n  (\n    second_argument +\n      third_argument\n  ) + last_argument\nlet y = 2\n",
        ),
        (
            "do\nf(first_argument+second_argument,{third_argument+fourth_argument,fifth_argument+sixth_argument})\nlet y=2\nend",
            "do\n  f(\n    first_argument +\n      second_argument,\n    {\n      third_argument +\n        fourth_argument,\n      fifth_argument +\n        sixth_argument\n    }\n  )\n  let y = 2\nend\n",
        ),
        (
            "let x=first_argument+ # note\nsecond_argument+\nthird_argument+fourth_argument\nlet y=2",
            "let x = first_argument +\n  # note\n  second_argument +\n  third_argument +\n  fourth_argument\nlet y = 2\n",
        ),
        (
            "match x\n| 1 => first_argument+second_argument+third_argument\n| _ => fourth_argument+fifth_argument+sixth_argument",
            "match x\n  | 1 => first_argument +\n    second_argument +\n    third_argument\n  | _ => fourth_argument +\n    fifth_argument +\n    sixth_argument\n",
        ),
        (
            "let x=first_argument+second_argument;let y=third_argument+fourth_argument",
            "let x = first_argument +\n  second_argument; let y = third_argument +\n  fourth_argument\n",
        ),
    ] {
        check(
            source,
            expected,
            FormatOptions {
                line_width: 24,
                ..FormatOptions::default()
            },
        );
    }
}
