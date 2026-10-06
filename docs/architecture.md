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

The [documentation handoff](verification/2026-10-06-documentation-handoff.md)
records current verification and review status. The
[architecture follow-up](verification/2026-10-06-architecture-followup.md), earlier
[final-check results](verification/2026-10-06-final-check.md) and
[architecture measurements](verification/architecture-deepening.md) are
stage-specific evidence, not measurements of every later change.

Admission is iterative and lexer-based, with counters shared across decoded
fragments. Possible `do` introducers consume layout units; their presence also
couples recursive prefix costs into that budget, including earlier fragments.
These charges stay separate from the AST traversal operator term. Ordinary
block-free operator/prefix accounting remains intact. The policy is verified
through public formatting and preservation on 2 MiB debug/release threads.

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

Neovim transport owns its subprocess pipes and deadline timer through public
`vim.uv` APIs. Completion closes the owned streams, so a wrapper's descendant
holding stdout or stderr cannot delay a timeout or abort a save. Process exit
and both output EOFs are required for normal success; cancellation, timeout and
errors settle once and ignore late callbacks.

The CLI owns syntax validation, resource admission, preservation, and
idempotence. Adapters own transmitting the current buffer and applying a
successful result without changing opaque bytes or overwriting newer edits.
No adapter writes files directly or introduces an alternate formatter.

The native VS Code test launcher observes both exit codes and termination
signals in its existing polling loop. A signal fails promptly with its name;
a zero wrapper exit still waits for an atomically published successful host
result. Detached-group cleanup and the host deadline remain in the same owner.
Controlled process regressions execute the existing launcher script directly;
the native `npm run test:host` suite exercises recognition,
automatic activation, unsaved buffers, edit application and idempotence.

The architecture scan retained the other module boundaries after a deletion
test. Removing `LayoutIndex` or `Doc` would spread containment and fitting rules
among callers. Removing editor transport, buffer codec or text-edit modules
would move process lifecycle and byte/position rules into their callers.
Provider/settings separation, CLI batch prevalidation, the preservation oracle
and the sequential editor runner had no demonstrated restructuring benefit.
A shared editor runtime would add dependencies without demonstrated leverage.
