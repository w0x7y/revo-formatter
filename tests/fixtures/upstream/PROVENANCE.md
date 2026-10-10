# Pinned upstream corpus

All original source comes from [if-not-nil/revo](https://github.com/if-not-nil/revo)
at revision `b571298b6fc95bc863548f118354c8d077792f6f`.
Upstream copyright is (c) 2026 lung & revo contributors. The complete unchanged
MIT notice is in [LICENSE.txt](LICENSE.txt). No fixture imports or executes a
Revo module during these tests; the bridge only lexes and parses source.
References to globals, intentionally wrong types, runtime errors and macro
bodies do not invalidate syntax. No type checking or macro expansion occurs.

This bounded selection contains 20 valid source fixtures: six whole examples
and fourteen self-contained documentation fences. The docs fixtures copy only
the code between the Markdown fences, with original bytes and final newline.
Line ranges are inclusive, one-based lines at the pinned revision. Whole
examples are exact copies, including their module comments. Nothing else from
upstream was selected for this corpus; there are no silent parse skips.

## Valid fixtures

Every row below parses successfully through the pure pinned parser.

| Local source fixture | Upstream path | First line | Last line |
| --- | --- | --- | --- |
| demo.rv | examples/demo.rv | 1 | 544 |
| pipes.rv | examples/pipes.rv | 1 | 67 |
| proc.rv | examples/proc.rv | 1 | 165 |
| types.rv | examples/types.rv | 1 | 185 |
| errors.rv | examples/errors.rv | 1 | 44 |
| misc-docs.rv | examples/misc/docs.rv | 1 | 16 |
| docs-annotations.rv | docs/docs.md | 356 | 359 |
| docs-type-alias.rv | docs/docs.md | 404 | 407 |
| docs-ambient.rv | docs/docs.md | 454 | 457 |
| docs-if.rv | docs/docs.md | 718 | 722 |
| docs-loops.rv | docs/docs.md | 733 | 760 |
| docs-labels.rv | docs/docs.md | 768 | 793 |
| docs-ranges.rv | docs/docs.md | 799 | 821 |
| docs-match.rv | docs/docs.md | 829 | 847 |
| docs-match-comma.rv | docs/docs.md | 853 | 856 |
| docs-doc-comments.rv | docs/docs.md | 1471 | 1481 |
| docs-comptime.rv | docs/docs.md | 1506 | 1509 |
| docs-proc.rv | docs/docs.md | 1531 | 1538 |
| docs-quotes.rv | docs/docs.md | 1596 | 1599 |
| docs-proc-quote.rv | docs/docs.md | 1623 | 1627 |

`demo.rv` includes double-quoted multiline strings, triple-quoted multiline
strings, module/documentation/line comments, interpolation, control flow,
types, methods, pipes and fibers. `proc.rv` adds tab-indented macro bodies;
`pipes.rv` includes multiline chains. The documentation fences cover typed
signatures, aliases, ambient declarations, if/else, labeled and unlabeled
loops, stepped/open ranges, match guards/comma arms, comments, comptime,
procedural macros and opaque quasiquotes. The corpus is evidence for these
examples, not exhaustive coverage of all accepted syntax.

## Rejected fixtures

| Local fixture | Upstream path and range | Result and reason |
| --- | --- | --- |
| `rejected/docs-type-name.rv` | `docs/docs.md:431-433` | Syntax error at byte 20, `':' without a following name is not a value; use ':name' for an atom`: upstream's fence contains truncated `pe MyInt = int`, diagnostic prose, and `t x = MyInt`. Copied unchanged; tested as a rejection at every option combination. |

One selected source fixture is rejected; no chosen source is excluded because
of external runtime or type dependencies. Malformed synthetic controls in
`src/tests/corpus.rs` are additional tests, not counted as upstream fixtures.

## Expected outputs

The four files in `expected/` are locally reviewed formatter outputs derived
from valid fixtures, not additional upstream inputs. They retain the same
MIT provenance. Suffixes specify line width and indent width.

| Expected output | Original fixture | Options | Checked layout |
| --- | --- | --- | --- |
| `docs-proc-24-2.rv` | `docs-proc.rv` | width 24, indent 2 | nested table reflow and macro block indentation |
| `docs-proc-120-4.rv` | `docs-proc.rv` | width 120, indent 4 | compact nested tables and macro block indentation |
| `docs-match-80-4.rv` | `docs-match.rv` | width 80, indent 4 | match-arm indentation and normalization of aligned spaces |
| `docs-ambient-24-4.rv` | `docs-ambient.rv` | width 24, indent 4 | multiline typed signature; expanded expression body receives its own indentation |

## Verification and limits

`src/tests/corpus.rs` checks every valid input at widths 24, 80 and 120 with
indent widths 2 and 4, both indent styles and blank-line limits 0, 1 and 2:
720 input/option combinations. Each result is checked for
reparsing, exact interleaved raw token/comment tape, complete AST equality
modulo source coordinates, and second-pass byte identity. There are four
expected-output cases, all at space indentation and blank-line limit 1, and one
selected invalid fixture checked at all 36 option sets. Direct whitespace and moved-comment/string-spelling controls prove
why both preservation checks are necessary. Production validator controls
also live in `src/lib.rs` and `src/oracle.rs`. See the
[current verification commands](../../../README.md#development-and-verification)
and [verification history](../../../docs/README.md#historical-verification).
After changing this provenance or a reviewed golden, update `SHA256SUMS` from
its existing file list and verify it from this directory.

Width is soft. Short expression newlines can collapse, and short block statements
inside pipe expressions can share a line. Expression bodies without `do` receive their own
indentation when expanded. Operator chains, including pipes, pack available
columns at one continuation indentation level. Unary minus stays attached to its
operand. Parser-sensitive label adjacency is retained, and a validated
conservative fallback may retain compact source spacing. Long unbreakable tokens
and expressions can exceed the target. Literal and comment bytes are never
reindented internally. Macro source coordinates
can change by the accepted position policy. This corpus executes no Revo code.
