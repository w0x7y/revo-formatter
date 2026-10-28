# Current architecture

This guide describes the implemented formatter as of 2026-10-28, with the
frontend pinned to Revo `eefa192`. The
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
   interleaved token/comment tape, and checks complete syntax tree equality
   modulo source coordinates.
5. The public formatter repeats the complete choice algorithm until it reaches a
   byte-identical fixed point, allowing at most four passes. Failure returns an
   error rather than an intermediate candidate.

Parsing does not load imports, resolve types or execute programs. Source
positions can change.

The pinned parser recurses with large frames up to its own nesting cap of 256
expression frames, about 127 nested prefixes. That needs more than a 2 MiB
thread, so the oracle runs every parse and comparison on a dedicated 16 MiB
frontend thread and joins it before returning. Admission still runs first, on
the caller's thread.

## Ownership

| Module | Responsibility |
| --- | --- |
| [src/lib.rs](../src/lib.rs) | Public option validation, preferred/conservative choice and fixed-point bound |
| [src/oracle.rs](../src/oracle.rs) | Source admission, owned FFI results, validated metadata, paired analyzed source and candidate preservation |
| [src/layout_index.rs](../src/layout_index.rs) | Indexed source facts, complete lexical envelopes and range-bounded typed scopes |
| [src/layout.rs](../src/layout.rs) | Scope traversal, document construction and, in its private gap module, every whitespace decision between tokens |
| [src/document.rs](../src/document.rs) | Cached flat widths, groups, fill continuations, enclosures, suffix fitting and rendering |
| [src/cli.rs](../src/cli.rs) | Argument grammar, bounded input, modes, per-input options from configuration and diagnostics. Its private `write_batch` module owns the `--write` lifecycle: preparing a batch checks and formats every input before any replacement, and only a prepared batch can be applied. Applying consumes it, replacing each changed file atomically in input order and reporting earlier completed paths on a later failure; the batch is not a transaction |
| [src/config.rs](../src/config.rs) | Range-validated layout flags, configuration discovery, bounded reading, TOML parsing and per-input option resolution. Flags are checked when supplied; file values only after precedence |
| [bridge/frontend.zig](../bridge/frontend.zig) | Synchronous C ABI, per-operation arenas and pure frontend calls |
| [bridge/source.zig](../bridge/source.zig) | The interleaved token/comment tape, with each complete string as one opaque token, and layout hints from exact CST token extents |
| [bridge/compare.zig](../bridge/compare.zig) | Exhaustive iterative structural comparison of concrete syntax trees, ignoring spans and token coordinates |
| [bridge/input_limits.zig](../bridge/input_limits.zig) | Lexer-only admission with global nesting and weighted complexity budgets |
| [build.rs](../build.rs) | Exact toolchain/platform checks and static bridge compilation |

## Layout boundaries

`LayoutIndex::scope(token, limit)` returns a complete construct only when its
lexical envelope fits inside the caller's token range. It owns scope priority and
the header, delimiter/generic, match and arm facts used by all three layout
traversals. Tokens remain the printing authority; source hints can overlap and
are not a complete surface syntax tree.

The pinned parser returns a lossless concrete syntax tree (CST) whose nodes
carry their significant tokens. The source collector takes each hint's extent
from the first and last token a node owns, so a `do` block's hint runs from its
`do` token to its `end` token and a statement's hint includes its doc comment.
It queues nodes while collecting tokens instead of using upstream's recursive
token walk, which exhausts a 2 MiB stack on long operator chains. Parsing
performs no lowering, so there are no generated wrappers. Interpolated string
contents stop traversal: the tape treats each complete string, including its
interpolations and any comments inside them, as one opaque token.

Rust consumes the `block` hints to seed block pairs and pairs only
`()`/`[]`/`{}` lexically. Neither owner balances raw `do`/`end` spellings, which
remain valid field names. The metadata interface is the seam between semantic
block ownership and layout.

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

Admission is iterative and lexer-based, with counters shared across the whole
source, including interpolated values. The oracle admits every analyzed source,
including generated candidates, before invoking the parser. Syntax tree
equivalence accepts only analyzed pairs. Private collector stress tests explicitly bypass admission for trusted,
shallow programs beyond the public token budget; that helper is not a second
production entrypoint. Possible `do` introducers consume layout units; their
presence also couples recursive prefix costs into that budget, including
earlier interpolations. These charges stay separate from the traversal operator
term. Ordinary
block-free operator/prefix accounting remains intact. The policy is verified
through public formatting and preservation on 2 MiB debug/release threads,
including the deepest nesting the parser accepts.

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
