# Formatting rules verification

Completed formatting-rules stage. Counts below were observed before the subsequent layout deepening and generic-call fitting fix. See the [final source check](2026-10-06-final-check.md) for later checks and the [current architecture](../architecture.md) for module ownership.

The user authorized all eight findings from the formatting review and the
nested twoSum example. The implementation fixes header/body grouping, generic
list locality, unary/index spacing, operator packing and coverage, compact
expression newlines, expression-function body indentation, and comment attachment.

Eleven new formatter regressions exercise the real frontend and public formatter.
Each checks expected bytes, exact token tape and AST preservation modulo source
coordinates, and idempotence. The initial seven tests failed before implementation.
Additional failing cases caught comparison/generic ambiguity, multi-parameter
loop headers, trailing list comments, and header width including `do`.

Reviewed expected-output changes compact short tables and parentheses, pack
arithmetic lines, keep operator comments attached, and indent the narrow ambient
fixture's function body. The existing 120 corpus/option combinations and resource
limit checks still pass. No vendor or public interface changes were needed.

Verification used the existing pinned Zig 0.17.0 executable:

- `cargo test`: 54 library tests, one binary test, 16 CLI tests; all passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo fmt --check` and `git diff --check`: passed.
- `zig test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter indexed`: six tests passed.
- `zig fmt --check bridge.zig bridge/*.zig`: passed.
- `cargo build --release`: passed.
- Release CLI reproduced the approved twoSum layout with compact headers and
  `return {y, x}`, including canonical `nums[y]` indexing. Reformatting returned
  identical bytes, and `--check` returned zero with no output.

An independent read-only review identified the comparison classification and
trailing-`do` width cases. Both were reproduced in failing tests and corrected.
