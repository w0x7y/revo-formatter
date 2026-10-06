# Formatting rules implementation plan

Completed implementation plan. The final review also corrected generic-call/list fitting and refreshed corpus metadata. Use the [current architecture](../../architecture.md) for delivered module boundaries and the [final source check](../../verification/2026-10-06-final-check.md) for verification.

> Executed inline in the feature worktree. The user authorized all findings from the formatting review.

**Goal:** Keep short headers and return values compact and correct the eight reproduced formatting issues.

**Architecture:** Retain the token tape, document renderer, source oracle, and conservative fallback. Extend source hints for real expression bodies and unary signs; give statements, headers, generics, and operator chunks independent layout groups. Use a fill-style document for operator continuations instead of measuring the entire remaining chain.

**Tech stack:** Rust, pinned Revo frontend compiled with Zig 0.17.0.

**Spec:** The user's twoSum example and the eight findings in this thread.

## Constraints

- Preserve exact token/comment bytes, AST structure modulo coordinates, and idempotence.
- Keep public options, CLI behavior, input limits, toolchain pins, and vendor source unchanged.
- Preserve parser-sensitive call, generic receiver, label, and range adjacency.
- Keep comments and opaque literals intact. Existing statement newlines remain boundaries.

## Tasks

- [x] Add failing end-to-end regressions in `src/tests/formatting.rs` for twoSum, short expanded returns, generics followed by statements, unary signs, indexing, operator packing/comparisons/pipes, short continuations, function bodies, and trailing comments. All use `check`, which verifies expected output, syntax preservation, and idempotence.
- [x] Extend `bridge/source.zig` and `src/layout_index.rs` with source-backed header/body and unary hints. Keep synthetic and opaque nodes excluded.
- [x] Fix spacing and independent formatting groups in `src/layout.rs`. Compact safe expression newlines, preserve statement boundaries and comment attachment, and retain conservative fallback for unsafe candidates.
- [x] Add continuation packing to `src/document.rs` and exercise it through real formatter cases, including narrow and wide output and nested scopes.
- [x] Review changed expected-output fixtures and tests against the approved policy. Update only intentional layout changes, retaining preservation checks and existing resource-limit regressions.
- [x] Update `README.md` to describe the new layout policy. Run `cargo test`, Zig bridge tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and a real CLI reproduction of the user's example. Request an independent review and address findings.

Commands selected an explicitly provisioned Zig 0.17.0 executable with `ZIG`.
For portable commands, see [development verification](../../../README.md#development-and-verification).
