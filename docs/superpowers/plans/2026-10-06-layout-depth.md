# Layout module deepening plan

Completed implementation plan. The final review also corrected generic-call/list fitting and refreshed corpus metadata. Use the [current architecture](../../architecture.md) for delivered module boundaries and the [final source check](../../verification/2026-10-06-final-check.md) for verification.

The user requested architecture improvements with direct implementation, skipping
the visual report and candidate-selection loop. Preserve the existing uncommitted
formatter fixes, public interface, syntax oracle, input limits, and toolchain pins.

The scan found concrete friction in the recently changed layout modules. Three
traversals independently orchestrate source scopes, match arms rediscover their
body seam by searching for textual arrows, and document suffix fitting requires
callers to know whether a document is already grouped.

- [x] Add failing formatter regressions for matches inside operator continuations
  and nested matches in guards. Add a document-interface regression proving that
  suffix fitting works for an ungrouped fill document.
- [x] Extend source-region metadata with source-backed match extents,
  subject/arm seams, and actual arm-head hints, retaining exclusion of opaque and generated descendants.
- [x] Deepen `LayoutIndex` with a typed, range-bounded scope interface that owns
  header/delimiter/generic/match/arm priority, lexical envelopes, and arm-body facts.
  Replace duplicated caller orchestration rather than adding another adapter.
- [x] Separate scope printing from expression traversal in `layout.rs`; make all
  three traversals consume the same scope query. Keep operator continuations
  inside their owning scopes and retain existing comment/layout policy.
- [x] Add `Doc::followed_by` to own grouping, suffix measurement, and composition.
  Remove the caller-facing reserve/group ordering contract. Encapsulate list
  grouping inside a document constructor rather than exposing `ungroup` to layout.
  Added an additional failing regression for wrapped match subjects: arm lists
  must not inherit subject continuation indentation.
- [x] Update the domain glossary and current architecture documentation. Run the
  Rust/Zig suites, lint/style checks, release build, and exact CLI regressions.
  Request independent review and fix concrete findings.

Tests cross the formatter/document module interfaces with literal expected output.
The new internal seam needs no adapter: all dependencies are in-process. The
existing preservation and resource-limit tests remain authoritative.
