use super::assert_preserved_and_idempotent;
use crate::{FormatError, FormatOptions, IndentStyle, format};

fn check(source: &str, expected: &str, options: FormatOptions) {
    let output = format(source, &options).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    assert_eq!(output, expected, "source: {source:?}");
    assert_preserved_and_idempotent(source, &output, &options);
}

#[test]
fn contextual_end_field_keeps_block_statements_indented() {
    check(
        "fn f() do\nfoo.end\nbar()\nend",
        "fn f() do\n  foo.end\n  bar()\nend\n",
        FormatOptions::default(),
    );
}

#[test]
fn contextual_do_binding_keeps_block_statements_indented() {
    check(
        "do\nlet do=1\nfoo()\nend",
        "do\n  let do = 1\n  foo()\nend\n",
        FormatOptions::default(),
    );
}

#[test]
fn contextual_end_binding_keeps_block_statements_indented() {
    check(
        "do\nlet end=1\nfoo()\nend",
        "do\n  let end = 1\n  foo()\nend\n",
        FormatOptions::default(),
    );
}

#[test]
fn contextual_keywords_inside_blocks_are_not_delimiters() {
    for (source, expected) in [
        (
            "fn f() do\nfoo.do\nbar()\nend",
            "fn f() do\n  foo.do\n  bar()\nend\n",
        ),
        (
            "do\nfoo. ## note ## end\nbar()\nend",
            "do\n  foo. ## note ## end\n  bar()\nend\n",
        ),
        (
            "do\nfoo. # note\nend\nbar()\nend",
            "do\n  foo. # note\n  end\n  bar()\nend\n",
        ),
        ("do/end\nfoo()\nend", "do /end\n  foo()\nend\n"),
        ("do/do\nfoo()\nend", "do /do\n  foo()\nend\n"),
        (
            "fn f(do,end) do\nfoo()\nend",
            "fn f(do, end) do\n  foo()\nend\n",
        ),
        (
            "fn f<do,end>() do\nfoo()\nend",
            "fn f < do, end > () do\n  foo()\nend\n",
        ),
        (
            "fn f.end() do\nfoo()\nend",
            "fn f.end () do\n  foo()\nend\n",
        ),
        (
            "do\nfor do,end in xs do\nfoo()\nend\nbar()\nend",
            "do\n  for do, end in xs do\n    foo()\n  end\n  bar()\nend\n",
        ),
        (
            "do\nloop/end do\nbreak/end nil\nend\nbar()\nend",
            "do\n  loop/end do\n    break/end nil\n  end\n  bar()\nend\n",
        ),
        (
            "do\ndo\nfoo.end\nfoo.do\nend\nbar()\nend",
            "do\n  do\n    foo.end\n    foo.do\n  end\n  bar()\nend\n",
        ),
        (
            "do\ndeclare x.end = number\nend",
            "do\n  declare x.end = number\nend\n",
        ),
        (
            "do\ntype X.end = number\nend",
            "do\n  type X.end = number\nend\n",
        ),
    ] {
        check(source, expected, FormatOptions::default());
    }
}

#[test]
fn final_nested_blocks_complete_their_owning_declarations() {
    for (source, expected) in [
        (
            "do\nlet x=do\nlet end=1\nend\nend",
            "do\n  let x = do\n    let end = 1\n  end\nend\n",
        ),
        (
            "do\ntest \"x\" do\nfoo.end\nend\nend",
            "do\n  test \"x\" do\n    foo.end\n  end\nend\n",
        ),
        (
            "do\nsuite \"x\" do\ntest \"y\" do\nfoo.do\nend\nend\nend",
            "do\n  suite \"x\" do\n    test \"y\" do\n      foo.do\n    end\n  end\nend\n",
        ),
        ("do/end end", "do /end end\n"),
        ("do do end end", "do do end end\n"),
        (
            "do\n1 |> do\nfoo.end\nend\nend",
            "do\n  1 |>\n    do\n      foo.end\n    end\nend\n",
        ),
    ] {
        check(source, expected, FormatOptions::default());
    }
}

#[test]
fn piped_long_block_bodies_keep_their_indentation_scope() {
    check(
        "do\n1 |> do\nfirst_function_with_long_name()\nsecond_function_with_long_name()\nthird_function_with_long_name()\nend\nend",
        "do\n  1 |>\n    do\n      first_function_with_long_name()\n      second_function_with_long_name()\n      third_function_with_long_name()\n    end\nend\n",
        FormatOptions::default(),
    );
}

#[test]
fn piped_nested_blocks_keep_their_hierarchical_scopes() {
    check(
        "do\n1 |> do\ndo\nfoo()\nbar()\nend\nbaz()\nend\nafter()\nend",
        "do\n  1 |>\n    do\n      do\n        foo() bar()\n      end\n      baz()\n    end\n  after()\nend\n",
        FormatOptions::default(),
    );
}

#[test]
fn nested_block_completion_and_generated_pipes_fit_a_two_mib_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [8, 16, 32] {
                let source = format!("{}foo.end{}", "do\n".repeat(depth), "\nend".repeat(depth));
                let output = format(&source, &FormatOptions::default()).unwrap();
                assert_preserved_and_idempotent(&source, &output, &FormatOptions::default());
            }
            // Lowered pipe blocks now contribute descendant completion while
            // retaining real block facts and suppressing synthetic hints.
            let source = format!("do {} do foo.end end end", "1 |> ".repeat(80));
            let output = format(&source, &FormatOptions::default()).unwrap();
            assert_preserved_and_idempotent(&source, &output, &FormatOptions::default());
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn compact_control_flow_headers_and_return_values() {
    let expected = "fn twoSum(nums, target) do\n  for y in 0..len(nums) do\n    for x in y + 1..len(nums) do\n      if nums[y] + nums[x] == target do\n        return {y, x}\n      end\n    end\n  end\nend\n";
    check(expected, expected, FormatOptions::default());
    check(
        "fn twoSum(nums, target) do\nfor y in 0..len(nums) do\nfor x in y +\n1..len(nums) do\nif nums [y] +\nnums [x] ==\ntarget do\nreturn {\ny,\nx\n}\nend\nend\nend\nend",
        expected,
        FormatOptions::default(),
    );
    for keyword in ["if", "unless", "while"] {
        let expected = format!("{keyword} x + 1 < 10 do\n  f()\nend\n");
        check(&expected, &expected, FormatOptions::default());
    }
}

#[test]
fn generic_lists_are_local_to_their_statement() {
    check(
        "f<T,U>(x)\nlet y=1",
        "f<T, U>(x)\nlet y = 1\n",
        FormatOptions::default(),
    );
    check(
        "f< T , U >(x)\nf<T,U>(x)",
        "f<T, U>(x)\nf<T, U>(x)\n",
        FormatOptions::default(),
    );
    check(
        "f<FirstType,SecondType>(x)",
        "f<\n  FirstType,\n  SecondType\n>(x)\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn short_generic_lists_do_not_expand_when_call_arguments_wrap() {
    for (source, expected) in [
        (
            "f<T>(first_argument,second_argument)",
            "f<T>(\n  first_argument,\n  second_argument\n)\n",
        ),
        (
            "f<T,U>(first_argument,second_argument)",
            "f<T, U>(\n  first_argument,\n  second_argument\n)\n",
        ),
        (
            "fn f<T>(first_argument,second_argument) first_argument",
            "fn f<T>(\n  first_argument,\n  second_argument\n)\n  first_argument\n",
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
    check("f<T>()", "f<T>()\n", FormatOptions::default());
    check(
        "receiver_methods<T>()",
        "receiver_methods<\n  T\n>()\n",
        FormatOptions {
            line_width: 20,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn unary_signs_and_indexing_stay_attached() {
    check(
        "let x=-1\nlet y=-value\nlet z=a*-b\nlet n=nums [y] [x]",
        "let x = -1\nlet y = -value\nlet z = a * -b\nlet n = nums[y][x]\n",
        FormatOptions::default(),
    );
    check("let x=a- -b", "let x = a - -b\n", FormatOptions::default());
}

#[test]
fn operator_continuations_pack_available_columns() {
    let narrow = FormatOptions {
        line_width: 24,
        ..FormatOptions::default()
    };
    check(
        "let x=value+value+value+value+value+value+value+value",
        "let x = value + value +\n  value + value +\n  value + value +\n  value + value\n",
        narrow,
    );
    for operator in [
        "<", ">", "==", "!=", "<=", ">=", "%", "^", "and", "or", "orelse", "|>",
    ] {
        let source = format!("let x=first_argument {operator} second_argument");
        let expected = format!("let x = first_argument {operator}\n  second_argument\n");
        check(&source, &expected, narrow);
    }
    check(
        "let x=first_value |> second_function |> third_function",
        "let x = first_value |>\n  second_function |>\n  third_function\n",
        narrow,
    );
}

#[test]
fn short_expression_newlines_can_compact() {
    for (source, expected) in [
        ("let x=1+\n2", "let x = 1 + 2\n"),
        ("return {\ny,\nx\n}", "return {y, x}\n"),
        ("f(\n1,\n2\n)", "f(1, 2)\n"),
        ("let x=(\n1+2\n)", "let x = (1 + 2)\n"),
    ] {
        check(source, expected, FormatOptions::default());
    }
}

#[test]
fn expression_function_bodies_are_visually_nested() {
    check("fn f(x)\nx+1", "fn f(x) x + 1\n", FormatOptions::default());
    check(
        "fn f(x)\nfirst_argument+second_argument",
        "fn f(x)\n  first_argument +\n    second_argument\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
    check(
        "fn f(x) # body\nx+1",
        "fn f(x) # body\n  x + 1\n",
        FormatOptions::default(),
    );
}

#[test]
fn operator_comments_remain_on_their_original_line() {
    check(
        "let x=1+ # note\n2",
        "let x = 1 + # note\n  2\n",
        FormatOptions::default(),
    );
    check(
        "let x=first_argument+ # note\nsecond_argument",
        "let x = first_argument + # note\n  second_argument\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn comparison_chains_are_not_generic_lists() {
    check(
        "let x=a < b > c",
        "let x = a < b > c\n",
        FormatOptions::default(),
    );
    check(
        "let x=first_argument < second_argument > third_argument",
        "let x = first_argument <\n  second_argument >\n  third_argument\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn multi_parameter_loop_headers_fit_independently_of_their_body() {
    check(
        "fn f() do\nfor x,y in items do\nf(x,y)\nend\nend",
        "fn f() do\n  for x, y in items do\n    f(x, y)\n  end\nend\n",
        FormatOptions::default(),
    );
    check(
        "for x,y in f(first_argument,second_argument) do\nf(x,y)\nend",
        "for x, y in f(\n  first_argument,\n  second_argument\n) do\n  f(x, y)\nend\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn header_width_includes_the_do_keyword() {
    check(
        "if first_argument + 22 do\nf()\nend",
        "if first_argument +\n  22 do\n  f()\nend\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn list_comments_remain_attached_to_the_preceding_item() {
    check(
        "return {\ny, # note\nx\n}",
        "return {\n  y, # note\n  x\n}\n",
        FormatOptions::default(),
    );
}

#[test]
fn nested_match_guard_uses_the_outer_arms_body_seam() {
    check(
        "match x\n| _ when (match y | _ => true) =>\nf(first_argument,second_argument)",
        "match x\n  | _ when (\n    match y | _ => true\n  ) =>\n    f(\n      first_argument,\n      second_argument\n    )\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn block_header_scopes_leave_post_block_operators_in_the_token_tape() {
    for head in ["if x", "unless x", "while x", "fn f()"] {
        check(
            &format!("{head} do\n1\nend+1"),
            &format!("{head} do\n  1\nend + 1\n"),
            FormatOptions::default(),
        );
    }
    check(
        "let x=if x do\nfirst_argument+second_argument\nend+1",
        "let x = if x do\n  first_argument +\n    second_argument\nend + 1\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn match_subject_continuations_do_not_indent_the_arm_list() {
    for (source, expected) in [
        (
            "match first_argument+second_argument\n| _ => third_argument+fourth_argument",
            "match first_argument +\n  second_argument\n  | _ => third_argument +\n    fourth_argument\n",
        ),
        (
            "let x=first_argument+match first_argument+second_argument\n| _ => third_argument+fourth_argument",
            "let x = first_argument +\n  match first_argument +\n    second_argument\n    | _ => third_argument +\n      fourth_argument\n",
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

#[test]
fn match_arms_own_continuations_inside_an_operator_operand() {
    check(
        "let x=first_argument+match y\n| _ => second_argument+third_argument\n| 1 => fourth_argument+fifth_argument",
        "let x = first_argument +\n  match y\n    | _ => second_argument +\n      third_argument\n    | 1 => fourth_argument +\n      fifth_argument\n",
        FormatOptions {
            line_width: 24,
            ..FormatOptions::default()
        },
    );
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
fn upstream_range_start_and_step_gaps_are_syntax_errors() {
    for source in [
        "for i in 0 ..3 do i end",
        "for i in 0\t..3 do i end",
        "for i in 0\n..3 do i end",
        "for i in 0 ## gap ##..3 do i end",
        "for i in 0..2 ..6 do i end",
        "for i in 0..2\n..6 do i end",
        "let start=0;for i in start ..3 do i end",
    ] {
        assert!(
            matches!(format(source, &FormatOptions::default()), Err(FormatError::Syntax { message, .. }) if message.contains("must be adjacent")),
            "{source:?}"
        );
    }
}

#[test]
fn upstream_unknown_interpolation_modes_are_syntax_errors() {
    for source in [
        "const t=1;print(\"#{t:d}\")",
        "const t=1;print(\"#{t:d  }\")",
        "const t=1;print(\"#{t:x}\")",
        "const t=1;print(\"\"\"\n  #{t:d}\n  \"\"\")",
    ] {
        assert!(
            matches!(format(source, &FormatOptions::default()), Err(FormatError::Syntax { message, .. }) if message.contains("doesnt work in interpolations")),
            "{source:?}"
        );
    }
    for source in [
        "print(\"#{}\")",
        "print(\"#{:v}\")",
        "print(\"#{:?}\")",
        "print(\"#{:p}\")",
    ] {
        assert!(
            matches!(
                format(source, &FormatOptions::default()),
                Err(FormatError::Syntax { .. })
            ),
            "{source:?}"
        );
    }
}

#[test]
fn upstream_interpolation_and_range_boundaries_preserve_programs() {
    for (source, expected) in [
        (
            "const t=42;print(\"#{t:v} #{t:?} #{t:p} #{:d}\")",
            "const t = 42; print(\"#{t:v} #{t:?} #{t:p} #{:d}\")\n",
        ),
        ("print(\"#{ :d }\")", "print(\"#{ :d }\")\n"),
        (
            "const t=42;print(\"100% complete: #{t:p}\")",
            "const t = 42; print(\"100% complete: #{t:p}\")\n",
        ),
        ("for i in 0..3 do i end", "for i in 0..3 do i end\n"),
        ("for i in ..3 do i end", "for i in ..3 do i end\n"),
        ("for i in 0..2..6 do i end", "for i in 0..2..6 do i end\n"),
        (
            "for i in 0.. do break i end",
            "for i in 0.. do break i end\n",
        ),
        (
            "for i in 0..2.. do break i end",
            "for i in 0..2.. do break i end\n",
        ),
        // A gap after the dots starts the body of an open-ended range.
        ("for i in 0.. break i", "for i in 0.. break i\n"),
        ("for i in 0..2.. break i", "for i in 0..2.. break i\n"),
        // The new left-adjacency check is specific to for-loop ranges.
        ("const r=0 ..3;print(r)", "const r = 0 ..3; print(r)\n"),
    ] {
        check(source, expected, FormatOptions::default());
        for line_width in [20, 24, 80, 240] {
            let options = FormatOptions {
                line_width,
                indent_width: 2,
                ..FormatOptions::default()
            };
            let output = format(source, &options).unwrap();
            assert_preserved_and_idempotent(source, &output, &options);
        }
    }
}

#[test]
fn opaque_and_record_doc_fixtures() {
    for (source, expected) in [
        (
            include_str!("../../tests/fixtures/formatting/opaque.rv"),
            include_str!("../../tests/fixtures/formatting/opaque.expected.rv"),
        ),
        (
            include_str!("../../tests/fixtures/formatting/record-docs.rv"),
            include_str!("../../tests/fixtures/formatting/record-docs.expected.rv"),
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
            ..FormatOptions::default()
        },
        FormatOptions {
            indent_width: 9,
            line_width: 80,
            ..FormatOptions::default()
        },
        FormatOptions {
            indent_width: 2,
            line_width: 19,
            ..FormatOptions::default()
        },
        FormatOptions {
            indent_width: 2,
            line_width: 241,
            ..FormatOptions::default()
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
                ..FormatOptions::default()
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
            "match x\n  | _ => {x = 1, y = 2}\n",
            "match x\n    | _ => {x = 1, y = 2}\n",
        ),
        (
            "match x\n| 1 => do\nf()\nend\n| _ => {\nx=1\n}",
            "match x\n  | 1 => do\n    f()\n  end\n  | _ => {x = 1}\n",
            "match x\n    | 1 => do\n        f()\n    end\n    | _ => {x = 1}\n",
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
                ..FormatOptions::default()
            },
        );
        check(
            source,
            expected_four,
            FormatOptions {
                indent_width: 4,
                line_width: 80,
                ..FormatOptions::default()
            },
        );
    }
    check(
        "match x\n| _ => {first_argument,second_argument}",
        "match x\n  | _ => {\n    first_argument,\n    second_argument\n  }\n",
        FormatOptions {
            indent_width: 2,
            line_width: 24,
            ..FormatOptions::default()
        },
    );
    check(
        "match x\n| _ => {first_argument,second_argument}",
        "match x\n    | _ => {first_argument, second_argument}\n",
        FormatOptions {
            indent_width: 4,
            line_width: 80,
            ..FormatOptions::default()
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
            ..FormatOptions::default()
        },
    );
    check(
        "match x\n| _ =>\nf(first_argument,second_argument)",
        "match x\n    | _ =>\n        f(\n            first_argument,\n            second_argument\n        )\n",
        FormatOptions {
            indent_width: 4,
            line_width: 24,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn statement_rhs_envelopes_compose_indentation() {
    for (source, narrow, wide) in [
        (
            "let x=first_argument+do\nf()\nend\nlet y=2",
            "let x = first_argument +\n  do\n    f()\n  end\nlet y = 2\n",
            "let x = first_argument +\n  do\n    f()\n  end\nlet y = 2\n",
        ),
        (
            "let x=first_argument+do\ndo\nf()\nend\nend\nlet y=2",
            "let x = first_argument +\n  do\n    do\n      f()\n    end\n  end\nlet y = 2\n",
            "let x = first_argument +\n  do\n    do\n      f()\n    end\n  end\nlet y = 2\n",
        ),
        (
            "let x=first_argument+{\na=1,\nb=2\n}\nlet y=2",
            "let x = first_argument +\n  {a = 1, b = 2}\nlet y = 2\n",
            "let x = first_argument + {a = 1, b = 2}\nlet y = 2\n",
        ),
        (
            "let x=first_argument+(\nsecond_argument\n)\nlet y=2",
            "let x = first_argument +\n  (second_argument)\nlet y = 2\n",
            "let x = first_argument + (second_argument)\nlet y = 2\n",
        ),
        (
            "do\nlet x=first_argument+(\nsecond_argument\n) # rhs\nlet y=2\nend\nlet z=3",
            "do\n  let x = first_argument +\n    (second_argument) # rhs\n  let y = 2\nend\nlet z = 3\n",
            "do\n  let x = first_argument + (second_argument) # rhs\n  let y = 2\nend\nlet z = 3\n",
        ),
        (
            "do\nmatch x\n| 1 => first_argument+do\nf()\nend\n| _ => second_argument+(\ng()\n)\nend\nlet y=2",
            "do\n  match x\n    | 1 => first_argument +\n      do\n        f()\n      end\n    | _ => second_argument +\n      (g())\nend\nlet y = 2\n",
            "do\n  match x\n    | 1 => first_argument +\n      do\n        f()\n      end\n    | _ => second_argument + (g())\nend\nlet y = 2\n",
        ),
    ] {
        for indent_width in [2, 4] {
            // Keep the compact parenthesized operand's available columns the
            // same when increasing the block and continuation indentation.
            for (line_width, expected) in [
                (if indent_width == 2 { 24 } else { 32 }, narrow),
                (80, wide),
            ] {
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
                check(
                    source,
                    &expected,
                    FormatOptions {
                        indent_width,
                        line_width,
                        ..FormatOptions::default()
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
    assert_eq!(
        output,
        "let x = value + value +\n  value + value +\n  value + value +\n  value + value\n"
    );
    assert_preserved_and_idempotent(&source, &output, &options);
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
    assert_preserved_and_idempotent(&source, &output, &options);
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
            "let x = first_argument + # note\n  second_argument +\n  third_argument +\n  fourth_argument\nlet y = 2\n",
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

#[test]
fn new_options_default_to_current_layout_and_validate_ranges() {
    let defaults = FormatOptions::default();
    assert_eq!(defaults.indent_style, IndentStyle::Space);
    assert_eq!(defaults.max_blank_lines, 1);
    for max_blank_lines in [0, 8] {
        assert!(
            FormatOptions {
                max_blank_lines,
                ..defaults
            }
            .validate()
            .is_ok()
        );
    }
    assert!(matches!(
        FormatOptions {
            max_blank_lines: 9,
            ..defaults
        }
        .validate(),
        Err(FormatError::InvalidOptions(_))
    ));
}
