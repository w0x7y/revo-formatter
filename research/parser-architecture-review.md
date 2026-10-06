# Parser architecture independent review

Historical research/review from 2026-10-05, covering the pinned revisions cited below. The coordinate-policy concern and research corrections were resolved before implementation; see [the re-review](research-rereview.md). The formatter and static bridge have since been implemented and tested. Use the [current architecture](../docs/architecture.md) and [documentation index](../docs/README.md) for delivered interfaces, commands and later verification. Original observations and review evidence remain below.

Reviewed on 2026-10-05 against upstream Revo revision `b571298b6fc95bc863548f118354c8d077792f6f`. Scope was `parser-architecture.md`, the formatter design spec, and the relevant pinned frontend and macro sources. No implementation, bridge compilation, upstream edits, or changes to the inspected documents were made. Two inline programs were executed with the matching installed `revo 0.1.2 (b571298)` to check a semantic concern.

## Research compliance verdict

Pass. The recommendation respects Rust CLI plus library, configurable indentation and width, first-release reflow, print by default, and the user's conditional approval of Zig parser reuse. It distinguishes the original Tree-sitter grammar from the compatibility fork, treats the compiler frontend as the syntax authority, records licensing and source-build obligations, and labels bridge compilation as future implementation work. The design requires independent review of every implementation task.

This review did not repeat remote revision queries, grammar binary probes, or the live Zig download-index lookup. Those historical observations remain attributed to the research rather than independently reproduced here. The checked local Revo revision and its MIT notice match the research.

## Research quality verdict

Needs one substantive correction before the proposed validation gate can support semantic safety. The bridge recommendation is feasible in principle, and most preservation details are grounded in the pinned source. However, excluding every `ast.Span` can hide values exposed to proc macros. This is a confirmed source and runtime issue, not an objection that the bridge has not been compiled.

## Confirmed semantic defect

High priority. Research lines 63 and 67 recommend excluding the exact `ast.Span` type, including `name_span`. Design lines 43-54 use the resulting comparison as the candidate safety gate. At this pin, some of those spans are observable program data.

[`FnParam.name_span`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/ast.zig#L303) and [`TypeExpr.span`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/ast.zig#L65) occur inside expression payloads. Proc macro [`encodePayload`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/macro_proc.zig#L500) and [`encodeValue`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lang/macro_proc.zig#L526) recursively serialize every struct field. The struct branch at lines 584-592 has no span exemption; the integer branch turns span coordinates into numbers. The special handling of `*Node` at lines 551-552 omits that node's own span, but does not omit spans inside its expression payload.

This valid program returns `86`:

```revo
proc offset!(iter) do let f = iter:next(); {{:number, f[1][0][1][0]}} end; offset!(fn(x) x)
```

Changing only `fn(x)` to `fn(  x)` returns `88`. The macro reads the parameter name's starting byte offset. The ordered token spellings and parsed structure excluding spans are unchanged, so the proposed gate would accept this behavioral change. Reformatting preceding text can also shift the exposed offsets. Keeping backtick contents opaque does not solve this universally: quasiquote inner positions are anchored to the token's position by `Parser.zig:1287-1293`.

Revise the guarantee and gate explicitly. If behavior preservation is required, account for macro-observable nested spans through conservative preservation or refusal to change affected input. Such a check can inspect trees without executing macros or resolving imports. If the intended guarantee is only syntax equivalence modulo positions, document the known macro limitation and obtain the user's agreement before relying on it. Merely adding more ordinary AST cases to a comparator that excludes every span does not fix this defect.

## Bridge and preservation assessment

The pure frontend entry exists at `Parser.zig:99-121`; it lexes and parses without invoking expansion, import loading, or compilation. Those phases appear later in `pipeline.zig`. Per-call arenas, an owned C result, and a matching free function are suitable implementation choices. `LICENSE.txt` permits reuse with its notice retained.

Complete structural comparison is implementable. The tree consists of tagged unions, structs, optionals, slices, pointers to child nodes/types, strings, enums, and scalar values. Compare pointer targets and slice contents, preserve union tags and order, and compare `f64` bits plus `is_float`. Every field matters, including `Node.synthetic_block`, declaration public/docs/kind, function attributes/defaults/generics, call `implicit_self`/`type_args`, record docs/optionality, match guards, labels, table `computed`, and quote splices. The research correctly rejects `Node.print`. The existing `ast.walkAST` is also unsuitable for exhaustive comparison: it prunes quasiquotes at line 1115 and skips many non-node fields.

The raw-range recipe is concrete and valid for top-level lexer origins. `lexComment` saves the opening line and byte column but returns body-only ranges for docs and module docs at `Lexer.zig:567-610`. Mapping that position into the original bytes and extending the body end over the asserted closing delimiter recovers the envelope. String/backtick ranges cover the full raw lexeme, while `Token.text` may contain decoded or dedented text. Slice original source bytes by raw ranges; never print `Token.text` as the preserved spelling.

Use one interleaved token/comment sequence or explicit comment anchors. Separate ordered lists of comments and code tokens would permit a comment to move past a code token while both checks still pass. AST equality cannot catch ordinary comment movement because `Parser.peek` skips comments and module docs. The research asks for the stronger tape; the design's “separate ordered” check should retain this meaning.

## Design risks to resolve at implementation boundaries

Medium priority. Design lines 38-39 promise lossless token ranges and syntax layout metadata “where useful”, but research line 61 specifies only original/candidate validation. Define an analysis result before the printer task begins, including ownership, token kinds/ranges, and the structural information Rust actually consumes. Desugared AST spans alone cannot supply all source regions: grouped-import blocks cover only the `import` token at `Parser.zig:1236`, parentheses disappear at `:1424-1430`, pipes synthesize/reorder calls at `:1832-1915`, and interpolation produces a synthetic `fmt` call at `:1997-2082`. Either return sufficient concrete annotations or specify the Rust syntax analysis needed to recover them. A validator can reject bad breaks but cannot supply statement and list grouping by itself.

Low priority. Make line-ending treatment explicit for line comments. `Lexer.zig:631-632` consumes a comment through the byte before LF, including a preceding CR. Appending a generic CRLF layout break after that unchanged raw comment would produce `CR CR LF`. Preserve the raw CR and emit only the missing LF in that case. Add a CRLF comment fixture when implementation starts.

The documented bare-call, call/generic adjacency, range-bound, and label hazards are real and correctly described. In particular, bare calls compare the callee's starting line with the next token at `Parser.zig:1477`; locking only the final argument gap is insufficient. The requirement for validation, stable output, and a conservative fallback is appropriate, but fallback must not quietly eliminate reflow for ordinary safely breakable lists and expressions. Expected-output tests should establish actual width behavior as well as accepted syntax.

## Unproven build work, not confirmed defects

Standalone static linking remains an implementation gate. The research accurately identifies indirect imports through `test_helpers`, diagnostic rendering, and type formatting. Declaration laziness may keep their runtime dependencies out of a production bridge, but this review did not compile that claim. The first implementation task should establish the required source closure, compatible pinned Zig toolchain, Linux link, allocation/panic handling, and Cargo packaging. No cross-platform claim is justified yet. Absence of a build experiment during explicitly read-only research does not fail research compliance.

The Rust library and CLI interfaces can be implemented as specified. The design should first correct the macro-span safety claim and settle the analysis interface used by the sequential printer task.
