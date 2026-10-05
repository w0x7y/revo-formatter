# Input admission policy

The public `format` boundary checks resource limits before copying source or
calling the parser. Every candidate passes the same check before analysis and
comparison. The CLI reads at most `MAX_SOURCE_BYTES + 1` bytes, checks length,
then decodes UTF-8. Options retain their existing validation order and defaults.
Source or candidate admission failures return `FormatError::Validation`; CLI
failures print no source and preserve every file in a prevalidated write batch.

| Shared budget | Maximum |
| --- | ---: |
| Source or candidate UTF-8 bytes (`MAX_SOURCE_BYTES`) | 262,144 |
| Cumulative decoded lexer source bytes | 1,048,576 |
| Non-EOF tokens across all lexer fragments | 4,096 |
| Combined punctuation and embedded fragment nesting | 32 |
| Parser score | 672 |
| Layout score | 800 |
| Syntax tree score | 1,536 |
| AST traversal score | 900 |

The private lexer-only bridge uses the pinned upstream lexer. It keeps an
iterative worklist of decoded interpolation and quasiquote fragments. Counters
are global across that worklist. Fragment nesting includes the surrounding
punctuation nesting. Literal punctuation and comment contents do not create
syntax depth. The interpolation scan follows the pinned parser's decoded open
indices, quote escapes, closing braces, and `:v`, `:?`, `:p` suffix handling.
Quasiquotes receive the parser's exact `%name` to `__qq_N` substitution before
lexing, including substitutions inside nested strings. Invalid lexical fragments
retain the normal parser diagnostics because the parser cannot recurse into
them before the same lexer rejects them.

Each score is a conservative lexical estimate, rather than a second parser:

- The parser score charges two per recursive prefix, declaration, unary,
  attribute/doc-comment, assignment/compound assignment, exponent,
  concatenation, `<`, `!`, or `?` token, plus four per maximum nesting level.
- The layout score charges one per pinned parser infix/logical operator, even
  when the builder does not reflow it, four per `match`, and four per maximum
  nesting level. Operators also charged by the parser score remain counted in
  both budgets: `<`, concatenation and exponentiation have type/associativity
  recursion in addition to their AST edges. Assignment and compound assignment
  are bounded by the parser score; ranges and pipes have separate wrapper costs.
- The syntax tree score counts syntax introducers, with two for declarations,
  opening parentheses/index brackets, attributes and quasiquotes; three for `fn`/`proc`;
  four for ranges; eight for pipes. Leaves, comments, closers, `end`, commas
  and semicolons cost zero. Other introducers cost one. An interpolated token
  additionally costs three plus its lexer-recorded open count.
- The AST traversal score starts with half the parser token score, the layout
  token score, and four per maximum nesting level. It adds one per field/index,
  ordinary string, possible hugging call, method selector, and
  `fn`/`proc`/`if`/`unless`/`match`/`loop`/`for`/`while`; four per range and eight
  per pipe. Interpolated tokens add three plus their open count. Direct anonymous
  and named `fn` signatures consume parentheses without extending receiver
  paths; contextual fields named `fn` or `proc` still count as potential calls.

The exact token sets and checks live in
[bridge/input_limits.zig](../../bridge/input_limits.zig). These global budgets
deliberately reject some large, shallow valid programs. Internal oracle metadata
tests use trusted source directly, preserving the existing 800-statement and
800-arm coverage. They are not an additional public entry point.

Thresholds were checked against every pinned fixture and the existing
800-operand formatting regression, with explicit 2 MiB Rust thread probes.
Those probes exposed independent parser, layout, and postfix AST traversal
stack exhaustion, requiring combined scores rather than delimiter-only or
independent per-form caps. The policy is tied to the pinned frontend and the
verified native Linux debug/release builds; a grammar or platform change needs
fresh resource-limit verification.
