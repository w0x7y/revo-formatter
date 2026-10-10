# formatter reference

Detailed CLI behavior, library use, preservation guarantees and corpus coverage.
For installation and examples, start with the [README](../README.md).

## CLI usage

```sh
# Read stdin and print formatted source. An explicit - also reads stdin.
printf 'let x=1' | target/release/revofmt
printf 'let x=1' | target/release/revofmt -

# Print one file without editing it.
target/release/revofmt example.rv

# Check or write several explicit files.
target/release/revofmt --check first.rv second.rv
target/release/revofmt --write first.rv second.rv

# Override layout settings.
target/release/revofmt --indent-width 4 --line-width 24 example.rv
target/release/revofmt --indent-style tab --max-blank-lines 2 example.rv

# A filename starting with a dash follows --.
target/release/revofmt -- --example.rv

target/release/revofmt --help
target/release/revofmt --version
```

For example, `let x=1` becomes `let x = 1` followed by a newline.
Width-driven formatting, tested with `--line-width 24`, changes:

```revo
print(first_argument, second_argument)
```

into:

```revo
print(
  first_argument,
  second_argument
)
```

Defaults are two spaces per indentation level, 80 display columns and at most
one consecutive blank line. `--indent-width` accepts 1 through 8;
`--line-width` accepts 20 through 240. `--indent-style` accepts `space` or `tab`;
tabs write one tab per indentation level and count `--indent-width` columns
when fitting lines. `--max-blank-lines` accepts 0 through 8.
Width is a soft target. Long literals, comments, and syntax that cannot safely
break can exceed it. The formatter can retain compact spacing through its
validated conservative fallback. Comment boundaries are retained. Short block
statements inside pipe expressions can share a line. Short expression
continuations, argument lists and return tables collapse onto one line when
they fit. Unary minus stays attached to its operand, and indexing uses `nums[y]`.

Control-flow headers are measured independently of their `do ... end` bodies.
Generic lists have their own formatting group; expanded lists put each argument
on its own indented line. Long call arguments can wrap while a short generic list
stays inline. Operator chains, including comparisons and pipes,
fill available columns with a single continuation indentation level. Trailing
operator comments stay attached to the preceding line. Function expression
bodies without `do` stay inline when short and receive indentation when expanded.
Parser-sensitive whitespace can still require conservative layout, and long
unbreakable expressions or comments can exceed the target.

Print mode accepts one input. `--check` also accepts stdin, including when
no input is specified. `--write` requires file paths and rejects stdin,
symlinks, and nonregular files. `--check` and `--write` are mutually exclusive.
There is no directory discovery or configuration file support in this version.

The CLI and library apply conservative input limits before parsing: 262,144
UTF-8 source bytes, 4,096 expanded lexer tokens, and 32 combined delimiter and
embedded-source levels. Shared weighted budgets also limit recursive parser
forms, AST traversal, and layout, including flat operators and postfix chains.
Possible source blocks share the layout budget with recursive prefixes and
operators, including combinations spread across decoded fragments.
Interpolation and quasiquote bodies count toward these budgets even though their
literal bytes remain opaque to formatting. Ordinary literal and comment contents
count toward the byte limit, without counting their punctuation as syntax.
Limits apply to generated candidates too; space for a final newline can therefore
be necessary. A limit failure returns `FormatError::Validation`, or CLI exit code
2 with no output or batch writes. CLI reads stop after one byte beyond the byte
limit. See [the exact admission policy](../docs/verification/input-limits.md).

Formatted source goes to stdout only in print mode. Check differences and
errors go to stderr with their input names. Exit codes are:

| Code | Meaning |
| --- | --- |
| 0 | Success, or all checked inputs already formatted |
| 1 | At least one checked input needs formatting |
| 2 | Usage, I/O, syntax, or validation error |

An error in any checked input takes precedence over code 1. Invalid syntax
produces no formatted source. Write mode computes and validates every input
before changing any file, so a malformed or unreadable input prevents all
writes. Each changed file uses a new temporary file in the same directory,
preserves the original permissions, flushes its contents, and replaces the
destination atomically. Unchanged files keep their existing inode and
modification time. Temporary files are removed on errors, with a diagnostic
if cleanup itself fails.

Multiple file writes are not a single transaction. A later replacement or
other I/O failure can leave earlier writes completed. The diagnostic lists
those completed paths so the result can be inspected.

## Rust library usage

Use this checkout as a local dependency:

```toml
[dependencies]
revofmt = { path = "/absolute/path/to/revo-formatter" }
```

```rust
use revofmt::{FormatError, FormatOptions, IndentStyle, format};

fn main() -> Result<(), FormatError> {
    let options = FormatOptions {
        indent_width: 2,
        line_width: 80,
        indent_style: IndentStyle::Space,
        max_blank_lines: 1,
    };
    let output = format("let x=1", &options)?;
    assert_eq!(output, "let x = 1\n");
    Ok(())
}
```

`FormatOptions::default()` supplies the CLI defaults. `FormatError` describes
invalid options, source syntax errors with byte offsets, or validation
failures. `UPSTREAM_REVISION` exposes the syntax revision.

## Preservation and validation

All non-whitespace token spellings and the interleaved order of tokens and
comments stay unchanged. Literal and comment contents remain opaque,
including documentation comments, quasiquotes, interpolation, and indentation
inside multiline strings. The formatter preserves layout line endings and
adds one final newline to nonempty source. Empty source remains empty.

Every returned candidate parses and has the same syntax tree after source
coordinates are excluded. The output is idempotent: formatting it again
produces identical bytes. This validates syntax without resolving imports,
checking types, executing Revo, or expanding procedural macros.

Formatting procedural macros is allowed. Macros that inspect offsets, lines,
or columns can observe formatting-induced position changes. The guarantee is
syntax equivalence modulo those coordinates, so such macros can produce
different results after formatting.

## Tested upstream corpus

The [vendored corpus provenance](../tests/fixtures/upstream/PROVENANCE.md) lists
20 valid historical inputs from revision
`b571298b6fc95bc863548f118354c8d077792f6f`: six complete `.rv` examples
and fourteen self-contained documentation snippets. It also lists one
malformed upstream documentation fence, which is tested as a syntax rejection.
The valid fixtures include the demo, pipes, procedural macros, types, control
flow, match arms, multiline literals and comments.

Each valid input is checked at line widths 24, 80 and 120 with indent widths
2 and 4, space and tab indentation, and blank-line limits 0, 1 and 2, giving
720 input/option combinations. Each combination produces one
formatter result, checked for reparsing, exact
interleaved raw token/comment bytes, complete AST equivalence modulo coordinates,
and idempotence. Four of those same results, at space indentation and the
default blank-line limit of 1, also check reviewed expected output
for signature/table reflow, block indentation and match-arm layout. The private
preservation and corpus tests run once in library test modules; process-level
CLI checks remain integration tests.
Negative controls cover malformed sources, whitespace-sensitive calls,
comment movement and literal respelling. This is a bounded regression corpus;
it does not establish exhaustive syntax coverage or uniform layout quality.

The separate [synthetic syntax stress corpus](../tests/fixtures/stress/README.md)
adds a readable 50-section file and reviewed default output. Whole-file,
isolated-section and composed-expression tests cover 77,184 valid input/option
combinations with LF/CRLF, six widths, four indent widths, both indent styles
and blank-line limits 0, 1 and 2. A separate
user-supplied guarded match with a typed table pattern adds 288 combinations,
for 77,472 total, using the same real preservation and idempotence checks. Invalid near misses and CLI
stdin/file/check/write behavior are tested separately. It uses the pinned
frontend at `f0034ab75aaf49d65bc1b4769987f99380383fcb`.
Gaps before loop-range dots and unknown interpolation mode suffixes are
rejected by that frontend. `:v`, `:?`, `:p` and lone atom `#{:d}` are covered
by complete-program regressions. Gaps after open-range dots can start loop
bodies, so formatting retains their adjacency decisions.

See the [architecture and Final Check record](verification/2026-10-09-architecture-final-check.md) for
current verification and review status, and the [earlier architecture report](../docs/verification/architecture-deepening.md)
for stage-specific package checks and measurements.

## Repository layout

| Directory | Contents |
| --- | --- |
| `src/` | Rust library, CLI, layout, and preservation tests |
| `bridge/` | Zig interface to the pinned frontend |
| `vendor/revo/` | Unchanged upstream source and checksums |
| `tests/` | CLI process tests and attributed corpus fixtures |
| `docs/` | Current guides, designs, plans, and verification records |
| `research/` | Dated upstream investigations |

Editor adapters and their development tooling live in
[separate repositories](editors.md).
