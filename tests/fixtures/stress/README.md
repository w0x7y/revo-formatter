# Synthetic syntax stress corpus

[syntax.rv](syntax.rv) is a handwritten, parse-only Revo file with 50 named
sections and an opening module comment. Do not execute it: names and imports
need not exist, and some loops intentionally have no upper bound.
[syntax.expected.rv](syntax.expected.rv) is its reviewed output with the default
formatter options. It retains all source tokens and comments.

The examples combine the pinned
[language guide](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md)
with parser-sensitive cases identified in the
[research note](../../../research/2026-10-07-revo-syntax-stress.md).
These are original synthetic examples, rather than copied upstream fixture
bytes. They do not change the attributed corpus or its checksum manifest.

Coverage includes numeric spellings, atoms, Unicode, multiline strings,
interpolation modes, all comment forms, binding and assignment operators,
tables, destructuring, fields, slices, ordinary/bare/method/generic calls,
function signatures, defaults, types, record docs, ambient and public
declarations, imports, native attributes, branches, matches, loops, labels,
ranges, contextual keyword names, pipes, comptime, fibers, procedural macros,
quasiquotes, tests and suites.

The complete file includes deliberately sensitive call, method, generic, range
and lexical adjacency. Its default output uses conservative layout and mainly
normalizes indentation. The isolated sections and composed expressions exercise
preferred reflow independently. Width remains a soft target.

The library tests in [corpus.rs](../../../src/tests/corpus.rs) check the complete
file, 51 isolated sections including module documentation, and 36 expressions
in six surrounding constructs. Every source runs with LF and CRLF at widths
20, 24, 40, 80, 120 and 240 and indent widths 1, 2, 4 and 8. This gives
12,864 valid input/option combinations. The separate user-supplied
[file-read match](match-file-read.rv) adds 48 combinations, for 12,912 total.
It combines propagation followed by a method call, guarded `|` arms with an
arrow on the next line, a typed table pattern and string interpolation; its
default layout is checked against literal expected output. Each result passes the real
interleaved token/comment tape and full AST preservation check, then a second
formatting pass must return identical bytes. Fifteen invalid near misses are
tested at all 24 option combinations, giving another 360 rejection checks.

The CLI process test checks stdin, repeated formatting, file printing,
`--check`, `--write`, and a final clean `--check` at three option pairs with
both LF and CRLF. It compares CLI bytes with the public library result.

With the exact local Zig toolchain configured as described in the project
[README](../../../README.md#development-and-verification), run:

```sh
cargo test --lib tests::corpus -- --nocapture
cargo test --test cli stress_syntax -- --nocapture
cargo build --release
target/release/revofmt tests/fixtures/stress/syntax.rv > /tmp/revo-stress.rv
target/release/revofmt --check /tmp/revo-stress.rv
diff -u tests/fixtures/stress/syntax.expected.rv /tmp/revo-stress.rv
```

The original combined draft exceeded the existing parser admission score.
Repeated examples were removed to admit the complete file without raising
resource limits. Invalid guessed syntax was moved into explicit rejection
tests, including a labeled `do` block immediately piped into a call, which the
pinned parser rejects. Neither was returned as damaged formatted source.

A finite corpus cannot cover every possible Revo program. These checks establish
syntax preservation and formatting stability for these inputs; they do not
typecheck, expand macros, resolve imports, execute programs, or establish
behavioral equivalence for macros that read source coordinates.
