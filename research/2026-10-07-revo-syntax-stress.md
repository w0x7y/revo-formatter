# Revo syntax stress-test research

Inspected 2026-10-07 for the user-requested formatter stress corpus. This note
covers the formatter's pinned frontend,
`b571298b6fc95bc863548f118354c8d077792f6f`, and does not update that pin.

## Primary sources and access

- [Pinned language guide](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md): the main language reference, including types, modules, documentation and macros.
- [Pinned async guide](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/async.md): fibers, channels and ordinary method/call examples.
- [Pinned parser](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig), [lexer](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig) and [type parser](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/type_syntax.zig): authoritative when prose or illustrative examples are ambiguous.
- The [official website](https://revo.lung.fyi/) was attempted through web access and direct HTTPS. Web access failed and HTTPS returned 403. Website contents were therefore not independently inspected. GitHub's pinned raw documentation was accessible; no runtime, toolchain or vendor update was fetched.
- The earlier [syntax inventory](syntax-inventory.md) and [review](syntax-inventory-review.md) provide detailed pinned source references; they are historical evidence, not a substitute for current verification.

## Coverage checklist

The language guide supports these families. Exact expected runtime values in
its examples are not formatter acceptance criteria.

| Family | Cases to exercise | Pinned guide location |
| --- | --- | --- |
| Bindings and tables | let, const, global, pub; positional/keyed/mixed tables; nested destructuring | [overview](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L63-L185) |
| Functions | anonymous/named/method definitions, closures, optional/default parameters, return annotations, empty blocks | [functions](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L211-L284) |
| Types | aliases, unions, optional/error sugar, table generics, records, function signatures, declare ambients | [types](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L346-L468) |
| Operators | unary, power, arithmetic, bitwise words, comparisons, logic, concat, assignment chains | [operators](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L612-L709) |
| Control | if/unless, nested loops, labels, bounded/open/stepped ranges, guarded/comma/table match patterns | [control](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L711-L888) |
| Composition | pipes, placeholders, method targets, closures, slices, propagation/fallback | [composition](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L890-L1089) |
| Other declarations | test, test/skip, suite, imports, declaration/module docs | [tests](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L1037-L1073), [modules/docs](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L1343-L1495) |
| Staging | comptime, proc declarations/calls, qualified macros, backtick captures, gensym | [advanced](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L1497-L1668) |
| Concurrency | spawn, yield, join and channel method calls | [fibers](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/docs.md#L1181-L1220) |

## Whitespace and opaque-content hazards

The following derive from the pinned parser/lexer rather than style preferences.

- Calls distinguish `f(x)` from `f (x)`; methods distinguish `t:foo(x)` from `t:foo (x)`. Preserve repeated calls and parentheses too. [Parser tests](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L2497-L2505).
- Generic calls require attached angle brackets and following parentheses: `id<num>(42)` and `m.id<T, U>(a, b)`. `f <T>(x)` and `f<1>(x)` instead parse as comparisons. [Generic tests](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L2538-L2559).
- Loop bounds after `..` require adjacency. A gap introduces the body of an open range. Exercise both ordinary and stepped ranges, negative steps, and leading `..`. Label slashes must touch the label. [Range/label parser](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1014-L1157).
- Bare string/table calls depend on line position. Comments can participate in raw-token adjacency decisions. Semicolons are explicit boundaries; replacing them with arbitrary newlines needs real preservation checks. [Bare calls](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1474-L1479), [statement boundaries](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1741-L1801).
- Keep colon annotation gaps (`x: int`), atom spellings, identifier `!`/`?` suffixes and punctuation boundaries. `//` is division. The attribute table accepts only `@native`. [Lexer](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig#L415-L498), [identifier/atom rules](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig#L1067-L1084), [attributes](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L582-L585).
- Preserve ordinary `#`/`##...##`, declaration `#*...*#`, and opening module `#!...!#` comments, including line endings and field-doc attachment. Module docs appearing after code are invalid. [Comment scanner](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig#L567-L633), [record docs](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/type_syntax.zig#L191-L225).
- Cover raw single quotes, escaped double quotes, interpolation and its modes, triple-quoted dedentation, multiline bodies, Unicode, and backtick captures. All literal bytes, including interior indentation, remain opaque. The scanner accepts multiline single-quoted strings. [Literal scanners](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig#L685-L1035), [interpolation](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1949-L2082).

## Pin differences observed

GitHub reported `main` at
[`a26356e671d509ad6453c9bb6f3554f1bb4e963b`](https://github.com/if-not-nil/revo/commit/a26356e671d509ad6453c9bb6f3554f1bb4e963b)
(committer date 2026-10-07T13:00:12Z). Direct byte comparison showed README,
async guide and lexer unchanged; language guide, parser and type parser differ.
This is a dated comparison, not an ongoing claim about main.

The newer [range parser](https://github.com/if-not-nil/revo/blob/a26356e671d509ad6453c9bb6f3554f1bb4e963b/src/lang/Parser.zig#L1072-L1123)
requires `..` to touch the preceding start/step, while the pinned parser does
not impose that check. Thus spaced starts/steps are useful pin-specific tests.
The newer parser also introduces `global_const` selection and removes
`looksLikeParenAssignStart`; do not import its syntax assumptions into this
formatter. The [guide diff](https://github.com/if-not-nil/revo/compare/b571298b6fc95bc863548f118354c8d077792f6f...a26356e671d509ad6453c9bb6f3554f1bb4e963b)
changes optional-argument sentinel explanations, truthiness, string API examples,
and incorrect range example comments. A source example may parse correctly
while its narrated runtime result is wrong.

## What verification establishes

Use the formatter's actual interleaved token/comment and complete AST
preservation interface, then format each result again and require byte equality.
Combine a readable broad file with isolated adjacency/literal/comment cases and
several widths/indents. Invalid-source probes should remain separate so one bad
example does not prevent the valid corpus from being exercised.

This tests the pinned syntax frontend and formatting stability. It does not
load imports, expand procedural macros, typecheck, execute tests or run a Revo
program. Coordinate-sensitive macros can observe changed positions even when
syntax preservation succeeds. See the implemented [syntax validation policy](../THIRD_PARTY.md#syntax-validation-policy)
and [public formatting contract](../src/lib.rs). A finite corpus cannot cover
every possible program; report its concrete families and matrix size.
