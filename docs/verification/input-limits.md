# Input admission policy

This is the current admission policy for the pinned frontend. See the
[architecture](../architecture.md) for call ownership and
[development checks](../../README.md#development-and-verification) for the full
Rust and local Zig suites, including the `input limits:` filter.
The [documentation handoff](2026-10-06-documentation-handoff.md) records current
verification; the [editor final check](2026-10-06-editor-final-check.md) records
the block-admission failure and its independently reviewed repair.

The public `format` boundary checks resource limits before copying source or
calling the parser. Every candidate passes the same check before analysis and
comparison. The CLI reads at most `MAX_SOURCE_BYTES + 1` bytes, checks length,
then decodes UTF-8. Options retain their existing validation order and defaults.
Source or candidate admission failures return `FormatError::Validation`; CLI
failures print no source and preserve every file in a prevalidated write batch.

| Shared budget | Maximum |
| --- | ---: |
| Source or candidate UTF-8 bytes (`MAX_SOURCE_BYTES`) | 262,144 |
| Lexer source bytes | 1,048,576 |
| Tape tokens: significant tokens outside strings, strings and comments | 4,096 |
| Combined punctuation and interpolation nesting | 32 |
| Parser score | 672 |
| Layout score | 800 |
| Syntax tree score | 1,536 |
| Syntax tree traversal score | 900 |

The private lexer-only bridge makes one iterative pass over the pinned upstream
lexer's output. That lexer emits interpolations inline, as `interp_open`, the
value's tokens and `interp_close`, so their tokens share every counter. Each
`interp_open` adds one nesting level on top of the surrounding punctuation.
Raw-string, string-part and comment contents do not create syntax depth. The
parser recovers after lexical errors and keeps parsing, so admission counts
tokens even when the lexer reports an error. The lexer-source cap is above the
public byte limit and only guards direct bridge callers.

Each score is a conservative lexical estimate, rather than a second parser:

- The parser score charges two per recursive prefix, declaration, unary,
  attribute/doc-comment, assignment/compound assignment, exponent,
  concatenation, `<`, `!`, or `?` token, plus four per maximum nesting level.
- The layout score charges one per pinned parser infix/logical operator, even
  when the builder does not reflow it, four per `match` or `do`, and four per
  maximum nesting level. When any lexical `do` occurs, it also charges two per
  `fn`/`if`/`unless`/`loop`/`for`/`while`/`not` token across the complete
  source. These prefix costs bound their combination with block
  document frames. Operators also charged by the parser score remain counted
  in both budgets: `<`, concatenation and exponentiation have type/associativity
  recursion in addition to their tree edges. Assignment and compound assignment
  are bounded by the parser score; ranges and pipes have separate wrapper costs.
- The syntax tree score counts syntax introducers, with two for declarations,
  opening parentheses/index brackets and attributes; three for `fn`;
  four for ranges; eight for pipes. Leaves, string tokens, closers, `end`,
  commas and semicolons cost zero. Other introducers cost one. A string with
  interpolations additionally costs three plus its number of `interp_open`
  tokens.
- The syntax tree traversal score starts with half the parser token score, the layout
  operator/`match` token score, and four per maximum nesting level. The layout
  block/prefix charges apply only to document construction. It adds one per
  field/index, string, possible hugging call, method selector, and
  `fn`/`if`/`unless`/`match`/`loop`/`for`/`while`; four per range and eight
  per pipe. Strings with interpolations add three plus their open count. Direct
  anonymous and named `fn` signatures consume parentheses without extending
  receiver paths; contextual fields named `fn` still count as potential calls.

The exact token sets and checks live in
[bridge/input_limits.zig](../../bridge/input_limits.zig). These global budgets
deliberately reject some large, shallow valid programs. Internal oracle metadata
tests use trusted source directly, preserving the existing 800-statement and
800-arm coverage. They are not an additional public entry point.

The `do` charge covers recursive block layout that punctuation depth cannot
bound. Every lexical `do` counts as a possible introducer, including contextual
field names, sibling blocks, and blocks inside interpolations. The check does not pair `do` with `end` or infer grammar. Ordinary
literal and comment contents stay opaque. Blocks share the layout budget with
operators, `match`, recursive prefixes, and maximum nesting, so a plain
`do ... 1 ... end` chain reaches the layout score at 200 blocks. Prefix counters
are global even before the first `do`; a later block also charges every earlier
prefix, including prefixes inside earlier interpolations. Sources
without lexical `do` retain their existing operator and traversal accounting.
Other forms consume the same global budgets and can lower the admitted block
count. The block/prefix layout charges do not alter the existing traversal
estimate. This deliberately restricts some shallow programs and combinations
of individually small interpolated values.

Thresholds were checked against every pinned fixture and the existing
800-operand formatting regression, with explicit 2 MiB Rust thread probes.
Those probes exposed independent parser, layout, and postfix syntax tree traversal
stack exhaustion, requiring combined scores rather than delimiter-only or
independent per-form caps. Ordinary blocks, blocks mixed with pipes, and blocks
inside interpolations are also checked near their shared admission boundary on
2 MiB debug and release threads. The policy is tied to the pinned frontend and
the verified native Linux debug/release builds; a grammar or platform change
needs fresh resource-limit verification.

## Revo `eefa192` frontend

At `eefa192` the pinned parser rejects nesting beyond 256 expression frames,
about 127 nested prefixes or 63 prefix-and-`do` pairs, with the syntax error
`expression too deeply nested`. Some admitted inputs, such as 200 nested
blocks, are therefore syntax errors rather than formatted results. Its frames
are also much larger than before: 2 MiB probes overflowed while parsing about
111 nested `fn()` prefixes, below that cap. Tightening these lexical budgets
enough to cover that would reject ordinary files with more than about 100
declarations, so the oracle instead runs every parse and comparison on a
dedicated 16 MiB thread. With that stack, every probed prefix form reaches the
parser's own cap. Admission, metadata collection and layout keep their
existing budgets, and the bridge's syntax tree walks for extents and comparison
queue nodes instead of recursing. The 2 MiB debug and release tests check the
existing boundary cases, which must now format or return the depth error, and
the deepest nesting the parser accepts for blocks, interpolated blocks, `fn`,
`not`, unary minus and `match`, which must format.
