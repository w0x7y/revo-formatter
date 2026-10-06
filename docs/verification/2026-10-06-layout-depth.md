# Layout scope and document module deepening

Completed layout-deepening stage. Counts below precede the final generic-call regression and fitting fix; the earlier differential measurements are retained as stage evidence. See the [final source check](2026-10-06-final-check.md) for later checks and the [current architecture](../architecture.md) for module ownership.

The user requested direct implementation of architecture findings and explicitly
skipped the visual report. The scan concentrated on recently changed layout code.
Existing frontend pins, public input limits, preservation checks, and the earlier
formatting fixes remain authoritative.

`LayoutIndex::scope(token, limit)` now returns a typed, complete layout scope
contained in the caller's range. It owns scope priority, lexical envelopes,
generic-call suffix composition, header/body seams, and match-arm facts. Sequence,
expression, and operand traversals share that interface. Scope printing lives in
one method; operator chunks no longer split inside a match operand's arms.

The source-region collector emits complete match hints, the first arm bar, and
actual arm-head arrows. Indexed arrow lookup uses the body span, so a nested match
in a guard cannot supply the outer arm's arrow. Grouping parentheses and comments
before a body remain part of its lexical layout. Match subjects and arm lists have
separate continuation scopes.

`Doc::followed_by` accepts any document and owns grouping, suffix measurement, and
composition. `Doc::enclosed` owns list grouping. Callers no longer arrange
`reserve_for` or `ungroup` operations in a representation-dependent order. The
renderer still uses cached group measurements, borrowed source text, display
columns, and independent operator chunk fitting.

Five added regressions crossed module interfaces and failed before their fixes:

- A suffix attached to an ungrouped fill reserves columns for `do`.
- A nested match guard uses the outer arm's real body seam.
- Match arms own operator continuations inside a surrounding operator operand.
- Wrapped match subjects do not indent their arm lists, including nested operands.
- Concrete block-header scopes retain every operator/token following `end`.

Formatter regressions assert literal expected output, token-tape and AST
preservation, and idempotence. Source metadata assertions were updated for the new
hints without weakening opaque/generated-node exclusion or scaling checks.

Verification used the pinned Zig 0.17.0 executable:

- `cargo test --all-targets`: 59 library, one binary, and 16 CLI tests passed.
- The corpus still passes all 120 fixture/option combinations with preservation
  and idempotence checks. Input admission and 2 MiB stack checks passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo fmt --check`, `zig fmt --check bridge.zig bridge/*.zig`, and
  `git diff --check`: passed.
- `zig test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter indexed`:
  six tests passed.

Independent review compared the pre-architecture snapshot with the implementation
across 180 generated sources at four widths. It exposed a scope stopping-point
regression for bodies such as `do ... end + 1`: printing stopped at the concrete
closer, while traversal consumed the longer AST body span. A failing regression
across conditional, loop, and function headers reproduced the defect. The index
now reports exactly the printed stop, leaving the suffix for the enclosing
traversal. Final review reran all 720 cases: both builds formatted the same 608
valid cases successfully, every current successful result was idempotent, and no
remaining concrete blockers were found.

`cargo build --release` passed. The release CLI produced exact expected layouts
for the original nested twoSum example, match operator operands, wrapped match
subjects, and a block-header suffix. Each of the four results formatted to
identical bytes a second time and returned zero from `--check` without output.
