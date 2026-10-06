# Parser architecture research

Historical parser-architecture research from 2026-10-05, covering the revisions
cited below. Recommendations, temporary paths and unproven build work record
that research stage; the static bridge and formatter have since been implemented.
Use the [documentation index](../docs/README.md) for current instructions and
verification, and the [architecture](../docs/architecture.md) for delivered
interfaces. Original source and runtime observations remain below.

Research date: 2026-10-05. Design research only; no implementation or checkout changes. The user selected Rust, CLI plus library, and line-width reflow in the first release while this investigation was running.

## Recommendation

Use a Rust library and CLI with an immutable-source token/trivia representation, a layout printer, and a small statically linked Zig bridge to the pinned upstream Revo frontend. The bridge should parse both input and candidate output and compare their complete normalized syntax structures before Rust returns formatted text. Zig is a source-build dependency, not a separately installed runtime dependency for distributed binaries. The user subsequently approved Zig parser reuse with Rust, subject to build and license feasibility. A small bridge build experiment must prove its dependency boundary and target linking before printer implementation relies on it.

The approved guarantee is complete syntax-structure preservation **modulo source positions**, together with exact interleaved token/comment bytes. Formatting may change offsets, lines and columns, including values visible to proc macros or source introspection. The user explicitly chose to allow formatting and document these as normal formatting effects. The comparator is a syntax-preservation gate; it does not guarantee identical behavior for programs that inspect source positions.

This is a recommendation inferred from the source, not a compiled prototype. If a Zig build dependency is unacceptable, implement a faithful lossless Rust parser following the pinned compiler's lexer, Pratt parser, and type parser. That is a larger first release. The available Tree-sitter grammar is useful for layout experiments but must not be the semantic authority. An indentation-only/gap-only release does not meet the requested line-width scope.

## Exact inspected versions

- Revo: clean `/tmp/revo-formatter-upstream`, origin `https://github.com/if-not-nil/revo.git`, HEAD `b571298b6fc95bc863548f118354c8d077792f6f`. `git ls-remote origin HEAD` matched. Local `revo version` reports `revo 0.1.2 (b571298)`.
- Original Tree-sitter grammar: Codeberg `doomy/tree-sitter-revo`, HEAD and `main` both `610fa6a4ff0fecd9cc81806e5e85ea61c92091b4`, verified with `git ls-remote` on 2026-10-05. The web reader could not open Codeberg; the clean local fork contains this exact history.
- Compatibility fork: clean `/tmp/revo-grammar-publication`, origin `https://github.com/w0x7y/tree-sitter-revo.git`, HEAD `f15165b5391656ed3dcce25e18dbfba4320d80ed`. It adds `67f839f21b1b65baa12c500e1e98702375ece10b` (current syntax/editor AST compatibility) and `f15165b` (isolated parser test builds) after the Codeberg HEAD. It is not the original upstream grammar. Diff includes 285 changed grammar lines and 218 scanner lines, generated sources, queries, and compatibility tests. Other nearby temporary checkouts were not modified or adopted.
- The existing `/tmp/revo-grammar-publication.so` was used for illustrative read-only parser probes. Its binary provenance was not rebuilt/verified in this task; observed results also follow the inspected grammar rules directly.

Pinned primary sources: [Revo tree](https://github.com/if-not-nil/revo/tree/b571298b6fc95bc863548f118354c8d077792f6f), [original grammar](https://codeberg.org/doomy/tree-sitter-revo/src/commit/610fa6a4ff0fecd9cc81806e5e85ea61c92091b4/grammar.js), [fork grammar](https://github.com/w0x7y/tree-sitter-revo/blob/f15165b5391656ed3dcce25e18dbfba4320d80ed/grammar.js).

## Actual alternatives

| Path | Useful existing component | Missing work and risk | Packaging |
|---|---|---|---|
| Reuse upstream Zig frontend behind Rust | `Parser.parseSourceReport` is explicitly a pure frontend entry, without baselib merging; lexer exposes tokens and positions. Exact compiler parsing rules. | AST loses ordinary comments, concrete parentheses, literal spelling and some surface forms; build a source-preserving printer and complete structural comparison. Prove standalone compilation. | Rust API/CLI plus static C ABI library compiled by Zig 0.17.0. No Revo executable required by shipped formatter. |
| Rust + Tree-sitter CST | Generated C parser/scanner, comments, byte ranges, punctuation; fork already fixes numerous syntax gaps. | Existing tree shape is wrong for ordinary expression precedence and bare calls. Requires significant grammar work or treating tree as untrusted layout hints behind compiler validation. Whitespace is not stored as a complete trivia tape; retain input bytes. | C compiler and Rust runtime binding at source build. Current grammar only enables Node bindings, not Rust: add a wrapper/build script or vendor generated C. Node/Tree-sitter CLI need only be regeneration tools, not end-user requirements. |
| Independent Rust lossless parser | Compiler source provides the actual token and parsing rules to port. | Largest maintenance burden; includes contextual type names, generic-call lookahead, adjacency, bare-call newlines, interpolation and quasiquotes. Porting just delimiters is insufficient for reliable statement layout. | Pure Rust deliverable, with pinned Revo parser oracle in development/CI strongly advised. |

The upstream source files contain 2,560 lines in `Parser.zig`, 1,629 in `Lexer.zig`, and 444 in `type_syntax.zig`, including tests and helpers. These counts indicate scope, not effort estimates.

Source references: [frontend entry](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L91), [AST](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/ast.zig), [Tree-sitter byte offsets/CST](https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html), [fork bindings configuration](https://github.com/w0x7y/tree-sitter-revo/blob/f15165b5391656ed3dcce25e18dbfba4320d80ed/tree-sitter.json).

## Concrete compatibility gaps

1. **Operator precedence is not compiler-compatible.** Fork `grammar.js:468` puts every `operation_expression` under one `prec.left(1)` rule. Its parse of `1 + 2 * 3` nests `1 + 2` as the left child of multiplication. Compiler `Parser.zig:1928–1932` assigns addition binding power 40 and multiplication 50; the installed matching Revo returns `7`. Both original grammar and fork share this limitation. A successful Tree-sitter parse or same-tree round trip cannot establish compiler equivalence.
2. **Bare calls are not represented as calls.** Fork parses `f "hi"` as sibling identifier and string nodes. Revo parses the string as a call argument on the same line in statement context. Demonstrated with the installed matching runtime:
   - `let f = fn(s) 7; let x = f "hi"; x` returns `7`.
   - Inserting one newline between `f` and `"hi"` returns `f()/1`.
   Fork trees give essentially the same sibling structure for both. The semantic condition in `Parser.zig:1477` compares the **callee's starting line** with the next token's line; protecting only the immediate gap before a bare argument is not enough if the callee itself can wrap.
3. **Accepting grammar errors differs from accepting language errors.** Fork accepts `type X = num\n#! late !#`; Revo rejects a module doc after code. `Lexer.zig:599` checks `some_seen`. Tree-sitter extras permit module docs globally. Error-free Tree-sitter trees are not syntax-validation authority.
4. **Original upstream lacks many fork repairs.** The fork adds grouped imports, ambient declarations, generic calls/path rules, numeric literal forms, compound operators, suffix/index support, immediate call parentheses, and type/record forms. Tests against the fork cannot be presented as tests against Codeberg HEAD.
5. **Call adjacency is fixed in the fork.** Fork `_call_parameters` uses `token.immediate('(')` (`grammar.js:312`), and probes correctly distinguish `f(1)` from `f (1)`. Do not carry an ordinary-call adjacency finding from the original grammar over to this fork.

Primary references: [compiler precedence](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1920), [bare-call rule](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1475), [module-doc lexer](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig#L567), [fork patch](https://github.com/w0x7y/tree-sitter-revo/commit/67f839f21b1b65baa12c500e1e98702375ece10b).

## Preservation requirements

- Preserve the ordered raw bytes of all non-whitespace tokens, ordinary/doc/module comments, strings, interpolation containers, backticks and numeric literals. Format their exterior gaps; do not decode and regenerate token text. Multiline strings are dedented by the lexer, so blindly reindenting their interiors changes values. Quasiquote bodies are parsed recursively and should remain opaque to the first printer.
- Preserve comment attachment and line-comment termination. Ordinary comments/module docs are skipped by `Parser.peek`; therefore AST equality alone cannot detect their deletion or movement. Independently compare one interleaved raw token/comment tape, including each kind and raw spelling: separate ordered lists would miss a comment moving across a code token. Doc tokens' upstream spans cover the body rather than necessarily the delimiter envelope; a Rust lossless scanner must retain its own complete raw span.
- Preserve CRLF line comments without doubling CR. `Lexer.zig:631–632` consumes through the byte before LF, so a raw line-comment token may already end in CR. When emitting a CRLF layout break after that unchanged token, emit only the missing LF; appending CRLF would produce CR CR LF. Include an exact-byte regression fixture.
- Protect lexical distinctions such as `:name` versus `: name`, identifier `?`/`!` suffixes, compound operators, and numeric forms. Re-lex candidate output and verify token kinds and raw bytes.
- Respect adjacency checks before call `(`, receiver-to-generic `<`, `>(`, `/label`, and loop range `..` to following expression. `for i in 0.. end_expression` uses spacing to distinguish an open-ended range from an explicit endpoint. Do not globally apply spaces around every operator.
- A conservative gap-only mode preserving all zero/nonzero gaps and every newline/non-newline boundary is plausible for indentation cleanup, but not arbitrary width reflow. Changing these properties needs parser-backed evidence. Token equality alone is insufficient.
- Preserve existing explicit parentheses, commas, semicolons, imports and declarations in order. Width is a layout target: long indivisible literals/comments or syntax that cannot safely break may exceed it.
- Reject invalid input; never emit a candidate failing the compiler parse or structural comparison. Fail without modifying a file if no safe candidate is produced. Verify deterministic output and `format(format(source)) == format(source)`; a candidate accepted once can still be unstable on the next pass.

References: [adjacency and calls](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L309), [range adjacency](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1093), [comment skipping](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Parser.zig#L1742), [multiline decoding](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/Lexer.zig#L807).

## Smallest practical Zig bridge

The proposed analysis and comparison contract was specified in [implementation plan, Task 1](../docs/superpowers/plans/2026-10-05-revo-formatter.md#task-1-build-the-pinned-frontend-bridge-and-rust-oracle). Its `analyze` returns an interleaved `SourceToken { kind, start, end }` tape and `SyntaxRegion { kind, start, end }` annotations in original UTF-8 byte coordinates. Proposed region kinds are `statement`, `block` and `match_arm`; exclude synthetic generated nodes and invalid ranges. Rust recovers remaining concrete grouping from source tokens. C ABI calls return owned structured JSON buffers with an explicit free function; Rust copies/deserializes and releases them on every path. `equivalent` implements complete structural equality modulo positions, separately from raw-tape preservation. The current architecture guide describes the later paired `AnalyzedSource` interface and richer source hints.

For an analysis ABI returning tokens and layout regions, return complete raw byte ranges separately from compiler diagnostic spans. `lexComment` retains the opening delimiter's line and byte column, but doc/module `Token.start/end` cover the body. A bridge can map the saved line/column into the original source for the raw start and extend the body end by two bytes for `*#` or `!#`, asserting both delimiters. Alternatively, add an explicit raw span in a small reviewed lexer adapter. Ordinary comments and strings already expose full lexeme ranges. AST-derived regions require care: desugaring creates synthetic nodes and some spans, such as grouped imports, do not cover the complete source construct. Source tokens remain the printing authority.

For the comparison call, receive original and candidate byte slices and return a structured result containing equivalence or diagnostics through the owned buffer contract above. Parse both into a per-call arena with `Parser.parseSourceReport(..., .{ .repl_mode = false })`, compare while arena memory is live, and free everything before returning except explicitly owned result data. Prefer direct structural equality over hashes; serialized canonical data is useful for tests/debugging. Expose no Zig pointers in the Rust public API. Avoid shared mutable parser state.

The comparison must traverse every tagged union case, field, optional and slice in `ast.Node`, `Expr`, `TypeExpr` and their child structures. Compare pointers by their contents, not allocation addresses. Under the approved position-change policy, exclude values of the exact `ast.Span` type, including `name_span`; preserve all other fields. In particular retain `synthetic_block`, declaration kind/public/docs, call `implicit_self` and `type_args`, parameter optional/default/variadic/type information, record optionality/docs, function attributes, match guards, labels, table `computed`, numeric `is_float`, macro names and quote splices. Use type-directed comparison or exhaustive serialization so new AST variants cause compilation/review work. Number comparison should preserve IEEE value bits and the float/int flag.

**Do not hash `Node.print`/`printPretty`.** They are debugging S-expressions, not a complete serialization: `.decl` prints only its inner node (`ast.zig:527`), function printing omits several parameter/attribute fields, and literals are decoded. Comparing these strings can miss meaningful changes.

The existing `ast.walkAST` is not an exhaustive comparator either: it prunes quasiquotes (`ast.zig:1115`) and does not visit every non-node field. Compare the complete data structures directly.

Parser desugaring appears deterministic at this pin: pipes use fixed `_` bindings (`Parser.zig:1915`), and quasiquote `__qq_N` numbering restarts per token (`:1268`). No node IDs exist in `ast.Node`. Removing spans avoids position differences under the approved guarantee without alpha-renaming real identifiers. Preserve the rest of the desugared structure exactly.

### Confirmed source-position behavior

The [independent review](parser-architecture-review.md#confirmed-semantic-defect) executed this program with matching `revo 0.1.2 (b571298)` and observed `86`:

```revo
proc offset!(iter) do let f = iter:next(); {{:number, f[1][0][1][0]}} end; offset!(fn(x) x)
```

Changing only `fn(x)` to `fn(  x)` produced `88`. The macro reads the parameter name's starting byte offset; ordered token spellings and syntax structure modulo positions remain identical. [`macro_proc.encodePayload`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/macro_proc.zig#L500) and [`encodeValue`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/macro_proc.zig#L526) recursively serialize struct fields, exposing nested `FnParam.name_span` and `TypeExpr.span` coordinates. Although `*Node` encoding omits the node's own span, it does not omit spans nested inside expression payloads. Quasiquote inner positions also depend on their enclosing token's position, so preserving raw backtick bytes does not prevent every observable offset change.

Under the user's explicit choice, formatting such code is allowed, and the comparator intentionally accepts the witness modulo spans. Document that macros/source introspection can observe formatting-induced position changes; do not call this validation a behavior-invariance guarantee or execute macros to validate output. This corrects the broader interpretation in the original research.

`Parser.parseSourceReport` handles proc definitions/calls and quasiquotes without invoking their expansion. Expansion, import preloading, type checking and compilation are later pipeline phases. The pinned lexer has a `macro` keyword but its parser prefix switch has no `.kw_macro` case; do not assume legacy macro declarations accepted by Tree-sitter are supported by the inspected compiler. A formatter validator should parse the pinned syntax and never execute user macros or load imports.

Dependency boundary still needs a build experiment. Direct frontend files reference `std`, `ast`, lexer, type syntax and diagnostics. Indirect imports include:

- `Parser -> test_helpers -> pipeline/revo` (test helpers).
- `diagnostic -> term -> revo` (rendering).
- `type_syntax -> compiler/types` (type formatting); that file imports `std` and `ast`.
- `ast` imports pipeline inside a test.

Zig's declaration laziness may leave these outside a production bridge that only parses and returns simple diagnostics; this was not compiled here. Start with a pinned source snapshot retaining layout, invoke a separate minimal bridge build rather than upstream `build.zig`, and prove which files/modules actually need inclusion. Do not claim the whole runtime dependency graph is required or that it is already avoided.

The inspected Revo revision specifies a minimum Zig version of **0.17.0** in `build.zig.zon` and requests `0.17.0` in README. For this formatter, require the **exact stable Zig 0.17.0** toolchain and reject development or other versions until separately reviewed. The [official download index](https://ziglang.org/download/index.json), checked on 2026-10-05, listed that stable release dated 2026-10-01, with host archives/checksums. Pin the archive and verify its checksum. No Zig executable was available in PATH during the original research. Rust builds would invoke the bridge build and link its static library; binary users need neither Zig nor Revo. Cross-target ABI/linking, panic/allocation behavior, and minimal imported modules remained build-validation work at this stage.

A subprocess oracle is simpler for initial comparison tests but is insufficient as the shipped safety gate unless the helper is bundled and always called. `revo compile` alone is not a syntax oracle: it invokes later phases and may expand macros/load imports. The installed CLI has no parse-only command, so a dedicated bridge/helper is still needed.

## License and packaging facts

Revo's pinned `LICENSE.txt` is MIT, copyright 2026 lung and Revo contributors. Preserve its notice when copying/porting substantial frontend code. Original Tree-sitter grammar declares MIT in `package.json` and the grammar header but has no root LICENSE file at the inspected commit; the fork adds an MIT notice crediting doomy/contributors and Idan Gilboa. Preserve that provenance if using the fork, and pin the chosen revision explicitly. Tree-sitter itself is [MIT licensed](https://github.com/tree-sitter/tree-sitter/blob/master/LICENSE). These are source facts and a packaging requirement, not a general legal assessment.

The Rust crate's published source must contain every pinned source/build input needed by its bridge build; do not depend on `/tmp` paths or fetching a moving branch in a consumer's build. Decide whether source builds require an installed pinned Zig or a documented toolchain provisioning step. Ship binary release artifacts for the initially supported targets.

Sources: [Revo license](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/LICENSE.txt), [fork license](https://github.com/w0x7y/tree-sitter-revo/blob/f15165b5391656ed3dcce25e18dbfba4320d80ed/LICENSE), [Revo toolchain pin](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/build.zig.zon).

## Proposed work seams after design approval

1. **Frontend bridge and comparator:** prove minimal Zig build; exhaustive AST comparison; diagnostics; invalid input; bare-call and adjacency adversarial fixtures. This is the architecture gate before printer work depends on it.
2. **Rust lossless source model:** raw tokens/trivia, protected literals/comments, lexical equivalence checker, bounded parser/layout annotations. Specify this interface before parallel implementation.
3. **Document/layout engine:** indentation and width-driven groups, grammar-aware statement/list breaks, preserved hard boundaries, candidate validation and deterministic safe fallback. An oracle makes bad output rejectable, but does not itself tell a printer where statements or lists belong.
4. **Rust library/CLI integration:** pure formatting API, options validation, stdin/stdout and file modes, atomic writes after validation, exit behavior and packaging. Keep path handling outside the formatter core.
5. **Independent verification:** pinned compiler examples plus dedicated semantic traps, snapshots, idempotence and normalized-AST comparison; review token preservation and AST comparator completeness separately.

Outstanding design/build decisions: proving the approved Zig source-build dependency works; initial target platforms; how much parser structure the Rust layout layer needs for complete first-release syntax coverage. The compiler validator prevents accepting known structural changes, but no compile experiment or complete semantics proof was performed in this research task.
