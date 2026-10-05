# Architecture deepening verification

Verified 2026-10-05 on native x86_64 Linux GNU, Rust/Cargo 1.99.0 and
exact Zig 0.17.0. Revo remains pinned to
`b571298b6fc95bc863548f118354c8d077792f6f`.

The private layout index owns complete delimiter envelopes and precomputed
statement/arm boundaries. Expression segments share continuation indentation;
list items and nested scopes reset it. Group widths are cached. The Zig collector
indexes raw token endpoints, opaque containment and concrete block closers.
`AnalyzedSource` pairs source text with metadata validated once and owns candidate
parsing, exact interleaved tape comparison and AST equivalence. Public library
and CLI interfaces, option bounds, four-pass fixed-point limit, conservative
fallback, FFI ownership and coordinate policy are unchanged.

## Review gates

| Gate | Status |
| --- | --- |
| Task 1, `layout_task_review` | Approved, no findings |
| Task 2, `bridge_task_review` | Approved, no findings |
| Task 3 | Independent review pending |
| Whole change | Final independent review pending |

This record reports implementation verification, not approval of the pending gates.

## Tests and package checks

The complete Rust suite passes **56 distinct tests, 56 executions**: 41 library,
one binary unit test and 14 CLI process integration tests. No failures or ignored
tests; zero doc tests. Oracle tests now run once, without recompiling the module
in integration binaries. The applicable standalone Zig suite passes **6 tests**,
including its root harness, two comparator controls and three token-index controls.
The filtered standalone harness excludes upstream runtime tests requiring the
unavailable `revo` module.

All 20 attributed valid fixtures remain in the unchanged **120-case matrix**,
covering widths 24/80/120 and indentation 2/4. Each combination formats once and
checks preservation and idempotence; four of those results also check reviewed
expected output. Malformed fixtures and every prior negative control remain.
See [corpus provenance](../../tests/fixtures/upstream/PROVENANCE.md).

Complementary preservation controls independently establish both predicates:
identical tape with changed AST, and equal AST with moved comments or respelled
literals. Additional controls cover invalid candidates, valid whitespace edits,
CRLF, Unicode, opaque interiors, source diagnostics, malformed JSON/schema,
missing fields, null bridge buffers and malformed token/region bounds. Tokens
must be ordered, disjoint and nonempty UTF-8 slices. Regions need valid bounded
UTF-8 slices and may be empty, overlapping, duplicated or unordered.

The controller's failure-coverage ruling accepts tests of the real private JSON
decoder and metadata constructor plus inspection of `preserves` error handling.
Only candidate Syntax errors become `false`; all other candidate analysis errors
and AST bridge errors propagate. Actual allocator exhaustion is not induced
deterministically. No mock adapters, injection hooks or public test interfaces
were added.

All Cargo checks use `--offline` where applicable, with `ZIG` set to the pinned
absolute executable and `CARGO_TARGET_DIR` set to the shared target directory:

```sh
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --offline --release
cargo package --offline --allow-dirty
zig test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter indexed
zig fmt --check bridge.zig bridge/compare.zig bridge/frontend.zig bridge/source.zig
```

All pass. Packaging verifies an extracted-source build; extracted library test
modules and fixture includes also compile. The 99 archive inputs contain all new Rust
modules and attributed fixtures, and exclude worktrees, agent scratch, local
tools and graph caches. The package has no source-path or scratch dependency.
Cargo's optional documentation/homepage/repository metadata advisory remains.
All 46 release CLI invocations and an external public-library consumer pass
with Zig unset and `PATH=/nonexistent`. Vendored source and corpus SHA256 manifests pass.

## Measured evidence

Task 1's renderer-only probe over 100/200/400/800 operands counted
59,302/238,602/957,202/3,834,402 baseline render-time width visits. With caching,
construction makes 595/1,195/2,395/4,795 visits and rendering makes zero.
Byte-for-byte renderer outputs match in 160 width/indent/newline/Unicode/opaque
combinations. Actual CLI output at those operand sizes is 596/1,196/2,396/4,796
bytes, maximum indentation two spaces, and a fixed point in every case.

Task 2's direct real-ABI before/after probe found identical ordered metadata for
all 20 fixtures: 4,835 tokens and 619 regions. Both serialized snapshots have
SHA256 `3485aec2155307b66b96e2f287b6e5498fbfd14a72dd0efca2b46fb407cc1ad3`.
Complete JSON responses also match for generated statements/arms at 200/400/800
items. End-to-end analysis medians over 21 calls after warmup were:

| Input | Items | Before ms | After ms |
| --- | ---: | ---: | ---: |
| statements | 200 | 1.611053 | 0.969812 |
| statements | 400 | 4.764137 | 1.977623 |
| statements | 800 | 15.659189 | 4.048841 |
| arms | 200 | 0.623931 | 0.500367 |
| arms | 400 | 1.667462 | 1.085019 |
| arms | 800 | 4.753843 | 2.203446 |

These host-specific measurements include parsing, collection and JSON transfer;
they are not isolated collector timings or performance guarantees. No timing
thresholds or instrumentation were added to product tests. Width remains soft,
opaque token contents remain unchanged, procedural macro coordinate effects are
accepted, and no runtime evaluation or import resolution is claimed.
