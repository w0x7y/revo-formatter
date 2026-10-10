# Current architecture

This guide describes the implemented formatter as of 2026-10-10. The
[domain glossary](../CONTEXT.md) defines its terms; the
[README](../README.md) defines public behavior and options.

## Formatting flow

1. `format` validates options through `FormatOptions::validate` and borrows the
   source until the oracle has admitted and parsed it.
2. The oracle owns admission and creates an `AnalyzedSource` pairing source text
   with validated token ranges and source-region hints from the pinned lexer/parser.
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
| [src/lib.rs](../src/lib.rs) | Public option validation, preferred/conservative choice and fixed-point bound |
| [src/oracle.rs](../src/oracle.rs) | Source admission, owned FFI results, validated metadata, paired analyzed source and candidate preservation |
| [src/layout_index.rs](../src/layout_index.rs) | Indexed source facts, complete lexical envelopes and range-bounded typed scopes |
| [src/layout.rs](../src/layout.rs) | Scope traversal, document construction and, in its private gap module, every whitespace decision between tokens |
| [src/document.rs](../src/document.rs) | Cached flat widths, groups, fill continuations, enclosures, suffix fitting and rendering |
| [src/cli.rs](../src/cli.rs) | Bounded input, modes, per-input options from configuration, diagnostics, batch prevalidation and atomic file replacement |
| [src/config.rs](../src/config.rs) | Configuration discovery, bounded reading, TOML parsing and per-input option resolution |
| [bridge/frontend.zig](../bridge/frontend.zig) | Synchronous C ABI, per-operation arenas and pure frontend calls |
| [bridge/source.zig](../bridge/source.zig) | Exact raw token envelopes, collector-owned block completion and source-backed layout hints |
| [bridge/compare.zig](../bridge/compare.zig) | Exhaustive structural comparison, ignoring only exact upstream `ast.Span` values |
| [bridge/input_limits.zig](../bridge/input_limits.zig) | Lexer-only admission with global fragment, nesting and weighted complexity budgets |
| [build.rs](../build.rs) | Exact toolchain/platform checks and static bridge compilation |

## Layout boundaries

`LayoutIndex::scope(token, limit)` returns a complete construct only when its
lexical envelope fits inside the caller's token range. It owns scope priority and
the header, delimiter/generic, match and arm facts used by all three layout
traversals. Tokens remain the printing authority; AST source hints may omit
punctuation or overlap, and are not a complete surface syntax tree.

The source collector owns one token-indexed table of complete block ends. It
proves an actual block from its AST provenance and source `do` opener, then
completes descendants child-first in the existing admitted traversal. For a
nonempty block, it finds the first lexical `end` at or beyond the completed
descendant extent; an empty block's span already includes its closer. This handles
final nested blocks and declaration wrappers whose own spans omit their bodies.
Missing closers fail metadata collection. A final linear pass extends applicable
block, statement and body hints from that same table, preserving hint order.

Generated wrappers still participate in descendant completion. Independently
proven source blocks beneath them emit `block` facts, including blocks on either
side of lowered pipes. Synthetic statement and header hints remain suppressed;
opaque literal and quasiquote interiors stop traversal. Generated ancestry alone
cannot hide a real block needed by layout.

Rust consumes the existing `block` hints to seed block pairs and pairs only
`()`/`[]`/`{}` lexically. Neither owner balances raw `do`/`end` spellings, which
the pinned parser also accepts as identifiers. The existing metadata interface
is the seam between semantic block ownership and layout. This gives the
collector greater depth and keeps block-envelope changes local to one owner.

Header fitting is independent of a following block body. Match subjects and arm
lists own separate continuations. Arm-body seams come from the actual source
arrow, so a nested match in a guard cannot supply the outer arm's boundary.
Flat operator chunks share one continuation indentation level; nested constructs
own their own levels. Scope traversal must still visit punctuation or suffixes
following a closing delimiter.

Within layout, a private gap module owns every whitespace decision between
adjacent tokens. Traversal states only the gap's intention: an ordinary join, an
expression join, a list join, an enclosure opening or closing, an
expression-body join or an operand continuation. The module gathers the gap's
original facts once and decides which rule wins: conservative layout, comment
attachment, the original newline count, statement and match-arm starts, and
parser-sensitive call, generic, label and range adjacency. It is the only place
that builds soft breaks or line breaks between tokens. Original newline counts
decide softening before the blank-line limit caps line breaks, so a list gap
capped to one break can compact on a later fixed-point pass. Operator chunk
discovery and scope traversal stay outside the module.

`Doc::followed_by` owns grouping and suffix measurement. `Doc::enclosed` owns list
fitting, allowing contents to fit after an independently rendered opener. A
generic call attaches its opening parenthesis to the generic list without
reserving the full call width. Empty calls reserve both parentheses. These rules
keep short generic lists and short argument lists compact independently.

## Changing the formatter

Keep syntax facts in the collector/index, scope traversal in layout, gap
decisions in layout's gap module, document fitting in the renderer, and
preservation in `AnalyzedSource`. A whitespace rule belongs in the gap module
under the intention it serves; traversal code states that intention rather than
building breaks. Gap regressions belong in the gap matrix in
`src/tests/formatting.rs`, which checks each row's preferred candidate as well
as its fixed point, so conservative fallback cannot hide a changed decision.
Exercise a changed interface through real callers instead of adding public test
hooks. The [contributor instructions](../AGENTS.md) identify test locations and
completion checks. The [admission policy](verification/input-limits.md) is the
authoritative budget reference; changing recursion or the grammar needs renewed
resource verification.

The [repository cleanup verification](verification/2026-10-07-editor-repository-cleanup.md)
records the current repository scope and checks. The
[documentation handoff](verification/2026-10-06-documentation-handoff.md),
[architecture follow-up](verification/2026-10-06-architecture-followup.md), earlier
[final-check results](verification/2026-10-06-final-check.md) and
[architecture measurements](verification/architecture-deepening.md) are
stage-specific evidence, not measurements of every later change.

Admission is iterative and lexer-based, with counters shared across decoded
fragments. The oracle admits every analyzed source, including generated
candidates, before invoking the parser. AST equivalence accepts only analyzed
pairs. Private collector stress tests explicitly bypass admission for trusted,
shallow programs beyond the public token budget; that helper is not a second
production entrypoint. Possible `do` introducers consume layout units; their
presence also couples recursive prefix costs into that budget, including earlier fragments.
These charges stay separate from the AST traversal operator term. Ordinary
block-free operator/prefix accounting remains intact. The policy is verified
through public formatting and preservation on 2 MiB debug/release threads.

## Editor integrations

Editor adapters are maintained in separate repositories:

- [Neovim](https://github.com/w0x7y/revofmt.nvim) owns its buffer lifecycle, byte codec, process transport and pinned binary installer.
- [VS Code](https://github.com/w0x7y/revofmt-vscode) owns its provider, text edits, process transport, native host tests and VSIX packaging.
- [Zed](https://github.com/w0x7y/revofmt-zed) owns its formatting language server, WASM launcher and pinned server downloads. A separate Revo language extension owns recognition and highlighting.

This repository owns the Rust library, CLI, pinned frontend and formatter tests.
It has no editor runtime or editor development dependencies. The
[integration guide](editors.md) defines the stdin/stdout contract and how to run
optional downstream checks against a rebuilt CLI.

The CLI owns syntax validation, resource admission, preservation and
idempotence. Adapters own transmitting the current unsaved buffer and applying
successful current output through the editor's representation. Their guides
define host-specific limits. Changes to that public boundary need verification
in the affected adapter repositories.
