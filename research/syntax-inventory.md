# Revo syntax constraints for a source formatter

Historical syntax inventory from 2026-10-05 for the pinned frontend cited below.
Source paths, line references and proposed checks record that research stage;
the static bridge and formatter have since been implemented. Use the
[documentation index](../docs/README.md) for current instructions and verification,
and the [architecture](../docs/architecture.md) for delivered interfaces.

Inspected 2026-10-05. Upstream is `https://github.com/if-not-nil/revo.git`, clean checkout at `b571298b6fc95bc863548f118354c8d077792f6f`, commit dated 2026-10-05. All source paths and line numbers below refer to that revision under `/tmp/revo-formatter-upstream`. The pinned [lexer](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig) was also fetched from GitHub's raw endpoint. The local source is the evidence for the detailed findings; this note does not claim those rules remain unchanged at future revisions.

The frontend toolchain pin is the official stable **Zig 0.17.0** release, not a development build or a floating minimum-version constraint. Its standard library implements `std.zig.string_literal.parseEscapeSequence`, called at `Lexer.zig:700`, `859`, and `1003`, so it is part of the literal-decoding dependency. Upstream requests 0.17.0 at `README.md:31` and `build.zig.zon:5`. The [official download index](https://ziglang.org/download/index.json), verified 2026-10-05, lists release date 2026-10-01. For this Linux x86_64 workspace, the exact archive is `zig-x86_64-linux-0.17.0.tar.xz`, SHA-256 `1cbe9df9f27e6b78d14ccbca43b6703a404ef79ef1c463de901d7f088d4e2026`. Other build hosts must pin their corresponding official 0.17.0 archive and checksum. This report records the intended dependency; no validator was built or tested here.

The formatter must retain concrete source structure. Revo's AST is already desugared and does not retain parentheses, ordinary comments, original string quoting, or all literal spellings. Equal token kinds and text alone do not establish equal syntax because the parser reads source adjacency and token lines.

## All parser decisions involving whitespace

The lexer skips ASCII whitespace without emitting newline tokens at `src/lang/Lexer.zig:393-404`; it records byte offsets, line, and column at `235-243`. LF increments the line counter at `540-551`. The following is the complete inventory of semantic offset/line checks in `src/lang/Parser.zig`, found by searching all `.line`, `.column`, `.start`, `.end`, `span`, and `tokenAdjacent` uses. Remaining coordinate uses establish spans or diagnostics. `src/lang/type_syntax.zig` has no coordinate-dependent syntax decision.

| Decision | Evidence in `src/lang/Parser.zig` | Constraint |
| --- | --- | --- |
| Method call `obj:method(args)` | 310-324, 1799 | Atom token and `(` must touch. There is no requirement for the receiver and atom to touch. |
| Generic call `f<T>(args)` | 337-356, 1623-1648 | `<` must touch the receiver's AST span end; `>` must touch `(`. Lookahead accepts only identifiers and commas and has a 32-token budget. |
| Ordinary/repeated call `f(args)` | 359-369, 1798 | `(` must touch the immediately preceding raw token's span end. |
| Optional loop/control label | 1014-1026, 1145-1157 | `/` must touch the following identifier. The parser does not require the keyword to touch `/`. Applies to `loop`, `while`, `for`, `break`, `continue`. |
| Open-ended `for` range | 1068-1122, helper 1139-1143 | The token after `..` is a bound only if adjacent. A gap makes it the body. This check also applies after the second `..` in a stepped range. Call sites also occur at 1079, 1091, 1111. |
| Bare string/table call | 1474-1479 | In statement mode, the next argument must start on the same line as the callee AST span's **start**, not its ending token. |
| Number followed by `(` | 1779-1785 | A nonadjacent `(` forces a statement boundary after a number. |

Tests at `Parser.zig:2497-2505` show the differences directly:

```revo
print("hi")     # call
print ("hi")    # block containing identifier and string
print
("hi")          # same block
(f)(1)          # call
(f) (1)         # block
t:foo(1)        # implicit-self method call
t:foo (1)       # block containing t, :foo, and 1
```

Generic tests at `2538-2559` distinguish `id<num>(42)` from `f <T>(x)`, which parses as `(f < T) > x`. Open-range examples follow the explicit rule at `1071-1072`:

```revo
for i in 0..10 do print(i) end
for i in 0.. do print(i) end
for i in 0..2..10 do print(i) end
for i in 0..2.. do print(i) end
```

A formatter must not put spaces after the `..` before `10`, or remove the space before the body of an open range. Ordinary infix ranges use a different parser path at `408-427`; slice ranges use `1583-1620`. Do not generalize the `for` rule to every `..`.

Bare calls accept strings, triple-quoted strings, and table literals, with callee kinds identifier, field, call, and anonymous function at `2115-2141`. For example, `f "x"` is a call in statement mode; moving `"x"` to the next line stops that bare call. The lexer emits no newline token, so most other expressions can continue across lines. Statement boundaries also depend on AST kind and the following token at `1772-1801`. Semicolons explicitly end expressions at `289` and separate block statements at `1535-1557`. Newlines are not a universal substitute for semicolons.

Preserve actual parser behavior around comments. `peek` and `peekAt` skip ordinary comments and module docs and mutate parser position at `1741-1763`, while ordinary-call adjacency uses `tokens[pos - 1]`. This can make a comment token participate in a positional decision. AST spans also differ from concrete ranges: parentheses return the inner node at `1424-1430`, and nonempty tables end their span at the last entry at `1390-1393`.

## Lexical hazards and sigils

`Lexer.zig:1067-1084` defines ASCII identifiers with `_`, digits after the first character, and trailing or interior `!` and `?`. Atom continuation additionally accepts `- + * / = < > . @ $ ~ ^ ? !`. Colon plus an identifier or symbolic atom starter becomes one atom token at `448-451`; a separated colon remains a colon token. Thus `:ok`, `:+`, and `:foo.bar` must remain intact. `$` outside an atom/string is an error at `477`. `@name` is one attribute token at `478-481`, `1051-1054`; the pinned parser recognizes only `@native` at `582-585`.

These pairs have different tokenization:

```revo
fn f(x: int) x       # separate ':' token for an annotation
fn f(x:int) x        # ':int' atom token, not the same annotation
value ?              # identifier plus propagation token
value?               # one identifier, a predicate-style name
check!(x)            # identifier ending in !, then call
```

Do not remove the annotation gap after `:`. The lexer also consumes `0x`, `0b`, `0o`, underscores, decimal/exponent punctuation, and hex float exponents at `640-682`. A dot following a number joins the number unless the next character is also a dot. Preserve numeric source spelling and token boundaries, including `1..3` versus decimal punctuation.

Maximal punctuation tokens at `415-498` include `|>`, `->`, `=>`, `==`, `!=`, `<=`, `>=`, `//`, `..`, and compound assignments. Removing spaces between independently lexed operators can combine them. `//` is division, not a comment. Word operators include `and`, `or`, `orelse`, `not`, `band`, `bor`, `bxor`, `shl`, `shr`.

## Comments and headers

The comment scanner at `Lexer.zig:567-633` recognizes:

- `#` to LF as an ordinary line comment.
- `## ... ##` as an ordinary block comment, ending at the first closing pair, without nesting.
- `#* ... *#` as a doc token. The expression parser attaches its trimmed body to the next declaration or method definition at `Parser.zig:632-654`. The type parser also consumes it before named, optional, and positional record fields, trims the body, and stores `RecordField.doc` at `type_syntax.zig:191-225`. Repeated field docs overwrite the pending doc, so the last one determines that field's semantic doc value.
- `#! ... !#` as a module doc token. It must precede the first noncomment token, or lexing fails with `LateModuleDoc` at `Lexer.zig:596-610`.

Preserve complete comment bytes and ordering. A newline terminating a line comment must remain a newline. Doc/module token spans cover body text rather than the whole delimiter pair at `590`, `606`, so token start/end ranges alone do not recover the concrete comment. Doc attachment is syntax, not trivia.

Include this record-field fixture in preservation tests, checking each field's doc and optionality as well as complete source bytes:

```revo
type Fields = {
  #* display name *# name: string,
  #* older note *# #* current note *# ?count: int,
  #* positional value *# number
}
```

The examples use `#!/usr/bin/env revo` as the opening of a module-doc block, closed later by `!#`, for example `examples/demo.rv:1-6`, `examples/pipes.rv:1-9`, and `examples/proc.rv:1-27`. The pinned lexer has no independent shebang exemption. Treating that first line as an ordinary standalone shebang would misread the intervening prose.

## Strings, interpolation, and macro templates

Preserve every literal from its opening delimiter through its closing delimiter as an opaque byte range in the first implementation. This includes internal indentation and closing-line whitespace.

| Form | Actual behavior and evidence |
| --- | --- |
| `"..."` | Escape decoding through Zig's escape parser, unknown escapes retained except `\#{`, interpolation with `#{...}`, doubled literal braces collapsed outside interpolation. Newlines are accepted. `Lexer.zig:685-774`; `examples/demo.rv:23-25`. |
| `'...'` | Entire body literal, without escapes or interpolation. Despite the function name `lexSingleLineString`, the scanner does not reject LF. `Lexer.zig:776-805`; interpolation test at `1146-1149`. |
| `"""..."""` | Escapes and interpolation like double quotes; dedentation if the **decoded** body starts with LF. `Lexer.zig:807-918`, `921-983`. |
| Backtick template | Escapes are decoded, including escaped backticks. The parser replaces `%name` captures with temporary names and reparses a quasiquote AST. It is not a general-purpose raw string. `Lexer.zig:986-1035`; `Parser.zig:1251-1297`. |

Multiline dedent reads only decoded body bytes. It never uses opening column or the lexer `line_start` state. At this revision `line_start` is declared at `383`, assigned at `546`, `549`, and never read. Moving indentation **before** an opaque triple-quoted literal does not alter its value. Editing indentation **inside** it can alter the value. Dedent removes the first LF, strips the minimum count of leading spaces/tabs from every nonempty body line, counts tabs as one byte, includes whitespace-only lines when finding the minimum, and removes trailing empty lines at `921-983`. Closing-line indentation is part of the body.

Interpolation is specifically `#{expr}`, not bare `{expr}`. `\#{expr}` and single-quoted `'#{expr}'` stay literal; tests at `Lexer.zig:1133-1149` cover those distinctions. A quote/braces state machine tracks nested contents at `185-226`. Embedded expressions are lexed and parsed again at `Parser.zig:1997-2053`; `:v`, `:?`, `:p` suffixes select modes at `1949-1956`, `2018-2025`. The parser desugars interpolation to a `fmt` call at `2070-2082`. Reprinting that AST would lose the user's literal and interpolation spelling. Preserve escaped nested quotes such as `"#{fn() \"nested\"}"` rather than processing the embedded expression with a naive outer-quote scanner.

## Function bodies, expression syntax, and precedence

Anonymous `fn(params) body`, named `fn name(params) body`, method `fn obj:name(params) body`, and field `fn obj.field(params) body` share `Parser.zig:657-748`. A body is one statement expression; `do ... end` explicitly groups multiple expressions. Optional parameters begin with `?`, annotations use a separated colon, defaults use `=`, and return types use `->` at `1482-1498`, `743`. Match arms use `| pattern [when guard] => expression` at `780-837`; there is no closing `end` for the match itself. `if`/`unless` parse a condition followed by one branch expression and optional `else` at `758-777`.

Named definitions desugar to declarations and method definitions insert implicit `self`. Pipes desugar to calls or synthetic blocks at `444-462`, `1832-1914`. Pipe anonymous bodies use a different minimum binding power at `452-454`, so body ownership needs parser support when reflowing chains. Representative real cases are `examples/pipes.rv:31-52` and `examples/demo.rv:184-189`.

Preserve parentheses in a concrete formatter. Assignment and compound assignment associate right at binding power 5. Pipes use 15. `or`, `orelse`, `and` use 10, 12, 20 at `2085-2091`. Comparisons use 30, arithmetic 40/50, bitwise operators 42/43/44/48, power is right-associative at 62/61, unary minus uses 60, `not` uses 35, bare calls use 70, and propagation uses 80 at `17-27`, `529-530`, `1920-1944`. The code's tables are more precise than prose about operator precedence.

## Defaults and validation requirements

The clearest explicit indentation setting is two spaces in the Helix configuration at [docs/editors.md:147](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/editors.md#L147). `examples/demo.rv:12-28` also uses two spaces. The language guide mixes styles. A read-only count over its 684 nonblank Revo fenced-code lines found raw leading-space widths `2:52`, `3:2`, `4:146`, `6:12`, `8:18`, plus 6 lines with tabs. Some four-space fences have an outer Markdown list margin, so those are not nesting-depth measurements. No source-formatting width default was found in README, CONTRIBUTING, or Markdown docs; the longest guide snippet line was 88 characters. Two spaces is source-backed; 80 or 100 columns would be a formatter design choice.

For a Rust CLI/library, a conservative whitespace-only rewrite can preserve every gap's empty/nonempty state, every LF sequence/count, and all concrete nonwhitespace token/comment/literal bytes. With a lexer matching upstream, those constraints preserve every parser branch listed above at this revision: offset equalities retain their truth values, lines stay equal or unequal, and opaque strings retain decoded values. This is a source-derived argument for syntax equivalence modulo positions, not full behavioral equivalence or a guarantee that a new scanner is correct. Such a formatter cannot fulfill unrestricted line-width wrapping because it cannot add/remove semantic line breaks or adjacency. Width should be a soft limit when syntax/literals prevent safe breaks.

Full reflow needs stronger validation:

1. Validate original and candidate with the pinned Revo revision and exact Zig 0.17.0 stable toolchain above, using `Parser.parseSourceReport` at `82-121`, `124-172` with the same options. Both `.err` reports and returned Zig errors are failures; a returned/recovered AST alone is not success.
2. Compare complete AST structure, retaining semantic fields such as `implicit_self`, float kind, optional/default parameters, generic arguments, declaration and record-field doc text/optionality, attributes, skipped tests, labels, and declaration kinds. Include `Node.synthetic_block`, which is outside `Expr` at `ast.zig:485-488`; the parser sets it for implicit top-level and grouped-import blocks at `Parser.zig:215`, `1237`, and prelude merging reads it to decide block flattening at `pipeline.zig:506-513`. Define any coordinate exclusions by exact field/type rather than deleting all numeric fields. A comparison excluding coordinates establishes only syntax equivalence modulo positions and needs the macro-coordinate policy below before serving as a behavior-preservation gate. An AST debug print is not a complete structural fingerprint: `ast.zig:644-662` omits optional/default parameter information, and prints are desugared forms.
3. Independently require exact original literal/comment bytes and retained concrete token spelling/order if the formatter's contract is whitespace-only. AST equality alone cannot protect ordinary comments, quote choices, parentheses, or numeric spellings.
4. Require idempotence, valid UTF-8/byte-offset handling, complete-input consumption, and non-destructive failure on unknown or malformed lexing. Test the distinctions above, comment-adjacent calls, single-expression anonymous functions, multiline/interpolated strings, labels, generics, and open ranges. Use upstream example fixtures as well as small isolated cases.

Frontend parsing avoids import resolution and compile-time execution. Compiling/executing arbitrary user code as a formatting check can run procedural macros and `comp` expressions; docs describe those at `docs/docs.md:1501-1503`, `1523-1526`.

## Macro-visible coordinates and accepted policy

Source coordinates are not exclusively diagnostic at this revision. `FnParam.name_span` and `TypeExpr.span` occur inside expression payloads at `ast.zig:303-305`, `65-67`. Proc macro `encodePayload` and `encodeValue` recursively serialize struct fields at `macro_proc.zig:501-517`, `526-594`. The special `*Node` handling at `551` serializes its expression rather than its outer node span, but the struct branch at `583-592` has no span exemption. Nested coordinate fields therefore become data available to user macros. Quasiquote inner positions are anchored to the backtick token position at `Parser.zig:1285-1293`; opaque template contents alone do not protect their absolute positions when preceding text changes.

The independent [parser architecture review](parser-architecture-review.md) reports executing this program with the matching installed Revo and obtaining `86`:

```revo
proc offset!(iter) do let f = iter:next(); {{:number, f[1][0][1][0]}} end; offset!(fn(x) x)
```

Changing only `fn(x)` to `fn(  x)` reportedly returned `88`, because the macro reads the parameter name's byte offset. This inventory verified the serializer source path but did not repeat runtime execution. The token spellings and AST structure excluding spans remain equal while behavior changes.

The user explicitly accepted: "Allow formatting; preserve syntax and document source-position changes as normal formatting effects." The formatter's contract is therefore syntax equivalence modulo positions, with the macro-coordinate limitation above documented as an accepted formatting effect. A comparator that discards spans supports that contract; it does not establish full behavioral equivalence for programs that inspect those coordinates.
