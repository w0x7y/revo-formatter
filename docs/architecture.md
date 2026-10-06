# Current architecture

This guide describes the implemented formatter as of 2026-10-06. The
[domain glossary](../CONTEXT.md) defines its terms; the
[README](../README.md) defines public behavior and options.

## Formatting flow

1. `format` validates options and admits the source before copying or parsing it.
2. The oracle creates an `AnalyzedSource` pairing source text with validated token
   ranges and source-region hints from the pinned lexer/parser.
3. Layout builds a document from complete token envelopes and renders a preferred
   candidate. If that candidate fails preservation, it tries conservative layout.
4. `AnalyzedSource::preserves` admits and parses the candidate, compares the exact
   interleaved token/comment tape, and checks complete AST equality modulo spans.
5. The public formatter repeats the complete choice algorithm until it reaches a
   byte-identical fixed point, allowing at most four passes. Failure returns an
   error rather than an intermediate candidate.

Parsing does not load imports, resolve types, execute programs or expand macros.
Source positions can change, including positions observable by procedural macros.

## Ownership

| Module | Responsibility |
| --- | --- |
| [src/lib.rs](../src/lib.rs) | Public options, source admission, preferred/conservative choice and fixed-point bound |
| [src/oracle.rs](../src/oracle.rs) | Owned FFI results, validated metadata, paired analyzed source and candidate preservation |
| [src/layout_index.rs](../src/layout_index.rs) | Indexed source facts, complete lexical envelopes and range-bounded typed scopes |
| [src/layout.rs](../src/layout.rs) | Token spacing and document construction within the owning scope |
| [src/document.rs](../src/document.rs) | Cached flat widths, groups, fill continuations, enclosures, suffix fitting and rendering |
| [src/cli.rs](../src/cli.rs) | Bounded input, modes, diagnostics, batch prevalidation and atomic file replacement |
| [bridge/frontend.zig](../bridge/frontend.zig) | Synchronous C ABI, per-operation arenas and pure frontend calls |
| [bridge/source.zig](../bridge/source.zig) | Exact raw token envelopes and indexed source-backed layout hints |
| [bridge/compare.zig](../bridge/compare.zig) | Exhaustive structural comparison, ignoring only exact upstream `ast.Span` values |
| [bridge/input_limits.zig](../bridge/input_limits.zig) | Lexer-only admission with global fragment, nesting and weighted complexity budgets |
| [build.rs](../build.rs) | Exact toolchain/platform checks and static bridge compilation |

## Layout boundaries

`LayoutIndex::scope(token, limit)` returns a complete construct only when its
lexical envelope fits inside the caller's token range. It owns scope priority and
the header, delimiter/generic, match and arm facts used by all three layout
traversals. Tokens remain the printing authority; AST source hints may omit
punctuation or overlap, and are not a complete surface syntax tree.

Header fitting is independent of a following block body. Match subjects and arm
lists own separate continuations. Arm-body seams come from the actual source
arrow, so a nested match in a guard cannot supply the outer arm's boundary.
Flat operator chunks share one continuation indentation level; nested constructs
own their own levels. Scope traversal must still visit punctuation or suffixes
following a closing delimiter.

`Doc::followed_by` owns grouping and suffix measurement. `Doc::enclosed` owns list
fitting, allowing contents to fit after an independently rendered opener. A
generic call attaches its opening parenthesis to the generic list without
reserving the full call width. Empty calls reserve both parentheses. These rules
keep short generic lists and short argument lists compact independently.

## Changing the formatter

Keep syntax facts in the collector/index, scope traversal and spacing in layout,
document fitting in the renderer, and preservation in `AnalyzedSource`.
Exercise a changed interface through real callers instead of adding public test
hooks. The [contributor instructions](../AGENTS.md) identify test locations and
completion checks. The [admission policy](verification/input-limits.md) is the
authoritative budget reference; changing recursion or the grammar needs renewed
resource verification.

The [final check](verification/2026-10-06-final-check.md) records the latest source
verification. Earlier [architecture measurements](verification/architecture-deepening.md)
are stage-specific evidence, not measurements of every later change.

## Editor packages

`editors/` contains independently installable adapters around the public CLI.
The Rust crate stays at the root and does not depend on any editor package.
Editor sources and their development dependencies are excluded from the Cargo
source package.

Each editor directory owns its metadata, source, tests, and installation guide.
Neovim separates buffer lifecycle from raw process transport and its buffer
codec. VS Code separates provider wiring from subprocess execution and text
edits. Zed uses declarative language registration and native external-formatter
settings. `editors/README.md` defines their shared stdin/stdout contract;
`scripts/verify-editors` invokes each package's checks.

The CLI owns syntax validation, resource admission, preservation, and
idempotence. Adapters own transmitting the current buffer and applying a
successful result without changing opaque bytes or overwriting newer edits.
No adapter writes files directly or introduces an alternate formatter.
