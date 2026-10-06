# Task 1 implementation report

Historical bridge implementation report from 2026-10-05. Task 2 and later formatter stages are complete; the temporary allowances and missing resource checks described below applied only to this stage. See the [final source check](2026-10-06-final-check.md) for later checks and the [current architecture](../architecture.md) for module ownership.

Status: implementation and task checks complete; ready for independent review.
No formatter, layout engine, CLI, macro execution, import resolution, or runtime
compilation was added.

## Delivered interface

`src/oracle.rs` implements the requested crate-private `analyze`, `equivalent`,
`Analysis`, `SourceToken`, and `SyntaxRegion` interface. `src/error.rs` exposes
`FormatError` with Display/Error. `src/lib.rs` exposes the exact pinned revision.
The module-level dead-code allowance is explicitly temporary: Task 2 consumes
this currently crate-private API and should remove the allowance then.

The C ABI returns an owned `{ ptr, len }` JSON buffer and has an explicit free
function. Rust's buffer guard frees it on every deserialization/error path.
Null/empty buffers become validation errors before slice construction. JSON
strings deserialize into owned Rust values while the Zig allocation is live.
Each bridge operation uses a fresh arena; nothing mutable is shared between
calls. Parse recovery errors are rejected even when upstream produced a tree.

Comparison recursively visits every struct field, tagged union tag/payload,
optional, array, slice, and pointed-to value. Exact `ast.Span` values alone are
ignored. Floats use bit equality; numeric integer/float kind is retained.
Unsupported future AST field types cause compile errors. No AST printer or
`walkAST` is used for equality.

Tokens omit EOF and retain source bytes including literal and comment
delimiters. For doc/module comments, the opening position comes from a
line-start table plus the lexer's saved byte column; closing delimiters are
validated at the body end. Ordinary comment CR bytes and UTF-8 byte offsets
are preserved. Regions are conservative source hints: real statement-list
children, concrete do/end blocks, and match arms starting after their bar.
Only valid lexical endpoints are emitted. Quasiquote/interpolation internals,
grouped import synthetic lists, and generated pipe blocks are excluded.

## Build boundary and provenance

- Upstream: `https://github.com/if-not-nil/revo`, revision
  `b571298b6fc95bc863548f118354c8d077792f6f`.
- Copied unchanged from `/tmp/revo-formatter-upstream`; that checkout was never
  modified. All `vendor/revo/` paths map directly to the same upstream paths.
- 22 Zig source files (relative-file import closure) plus the upstream license
  are retained. `REVISION` and `SHA256SUMS` are locally generated metadata.
- Zig resolves imports in otherwise unused declarations/tests, so five core
  frontend files alone fail with missing pipeline/test-helper/type/term paths.
  The recursive unchanged import closure compiles without named runtime module
  stubs, upstream build.zig, or optional runtime dependencies. Only lexer/parser
  declarations are called by the bridge; presence of pipeline source is not
  use of the pipeline.
- Added root `bridge.zig` as a tiny module entry so both handwritten bridge and
  unchanged vendor imports live inside one Zig module's root directory. This
  is an additional build-input file beyond the brief's illustrative file list.
- Exact stable Zig 0.17.0 is checked, discovered from `ZIG` or PATH. All Cargo
  checks below explicitly set `ZIG` to the provisioned local compiler. No
  download/global installation logic exists. The supplied Linux archive pin
  was already checksum-verified before this task; THIRD_PARTY.md records it.
- Native HOST/TARGET `x86_64-unknown-linux-gnu` only; Zig receives explicit
  `x86_64-linux-gnu`, baseline CPU, PIC, ReleaseSafe, libc, and compiler-rt.
  `-fcompiler-rt` resolved the initial Rust link's missing `__zig_probe_stack`.
  The archive has no separately installed Zig/Revo runtime dependency.
- `build.rs` rerun inputs contain bridge/vendor sources and ZIG, never generated
  target output. `.gitignore` already had `/target/` and `/.tools/` at baseline,
  so no redundant edit was made.

## Red/green evidence

1. Before implementing the bridge, four Rust behavior tests ran against
   explicit `bridge not implemented` stubs. `cargo test --lib`: **0 passed,
   4 failed**. The failure log was the local development artifact `task-1-red.log`. Acceptance,
   error classification, structural change detection, and concurrent ownership
   all failed for the missing behavior.
2. After the initial bridge: **4 passed, 0 failed**.
3. Metadata assertions were added while the bridge still emitted upstream doc
   body spans and no regions. `task-1-metadata-red.log` records missing region
   and complete-comment-envelope failures. One additional fixture failure was
   invalid upstream syntax, corrected as described below. Then **8 passed**.
4. A targeted generated-block case (`do 1 end |> 2`) exposed a generated block
   reusing a concrete block start; malformed `proc () 1` exposed upstream errors
   escaping ParseResult. `task-1-edge-red.log`: **7 passed, 2 failed**. The region
   guard now rejects an AST extent beyond its matched concrete end; escaped
   parser errors now map to Syntax rather than Validation. Final **9 passed**.
5. Supplemental direct Zig tests verify exact Span exclusion, preservation of
   synthetic_block and a non-Span field literally named `span`, signed-zero/NaN
   bit distinctions, and content-based equality across separate allocations.

Pinned syntax fixture corrections: `loop/one do break/one end` was rejected by
this pin; use `loop/one do break/one nil end`. Anonymous `fn<T>(x: T) x` was
rejected; the accepted generic function type fixture is
`declare f = fn<T>(x: T) -> T`. Generic call coverage uses `f<T>(1)`.

Rust coverage includes precedence, bare-call newlines, `f(1)`/`f (1)`, generics,
open ranges, labels, optional/default/typed/variadic parameters, public and
ordinary declaration kinds, record optionality, doc fields, implicit-self
calls, numeric kind, trailing/recovery errors, empty input, exact comment/raw
string spans, generated region exclusion, the accepted proc-coordinate
witness, null ABI buffers, and 8 concurrent workers with 30 repeated rounds.

## Exact final checks

Run from `/home/idan/GitRepo/revo-formatter`:

```sh
ZIG="$PWD/.tools/zig-x86_64-linux-0.17.0/zig" cargo test
# 9 Rust unit tests passed; 0 doc tests; no failures.
cargo fmt --check
# Passed.
ZIG="$PWD/.tools/zig-x86_64-linux-0.17.0/zig" cargo clippy --all-targets -- -D warnings
# Passed, no warnings.
.tools/zig-x86_64-linux-0.17.0/zig fmt --check bridge.zig bridge/*.zig
# Passed.
.tools/zig-x86_64-linux-0.17.0/zig test bridge.zig -lc --test-filter 'bridge:' --cache-dir target/zig-test-cache
# 3 passed (2 named comparator tests plus import harness).
git diff --cached --check -- . ':!vendor/revo'
# Passed for handwritten/provenance/report files.
```

The full staged whitespace check reports two existing trailing-space lines in
upstream Lexer.zig test string fixtures (lines 1547 and 1558). These bytes are
intentionally unchanged and checksum-identical; the handwritten check passes.

Also executed the actual Cargo build-script binary with controlled HOST,
TARGET, and ZIG variables: unsupported aarch64 target, nonexistent Zig, and a
program returning a non-0.17.0 version all exited unsuccessfully with the
expected actionable diagnostics. Independently recomputed every manifest
SHA-256 and compared file bytes against the clean upstream checkout:
**23/23 unchanged upstream files passed**.

## Files

Original code/build/provenance: `Cargo.toml`, `Cargo.lock`, `build.rs`,
`bridge.zig`, `bridge/frontend.zig`, `bridge/compare.zig`, `bridge/source.zig`,
`src/lib.rs`, `src/error.rs`, `src/oracle.rs`, `LICENSE`, `THIRD_PARTY.md`,
`vendor/revo/REVISION`, `vendor/revo/SHA256SUMS`.
Vendored source/license file list is exactly the path list in
`vendor/revo/SHA256SUMS` (22 Zig files and LICENSE.txt).
This report is maintained at `docs/verification/bridge-build.md`. Red logs were
local development artifacts; they are not shipped beside this maintained copy.

## Concerns and review notes

- Regions are deliberately incomplete hints: parser spans can omit punctuation
  and generated constructs can hide real nested layout opportunities. Tokens
  remain the complete printing authority. Task 2 must not treat region spans
  as a complete surface AST or use them to drop punctuation.
- A small class of upstream parser errors lacks a diagnostic span. These return
  a Syntax error with upstream error-name text and offset zero; reported errors
  retain their real byte offset.
- No cross-platform support claim is made. Native Linux x86_64 is the only
  tested/accepted source-build target.
- This is syntax equivalence modulo coordinates; ordinary comment/module-doc
  tape equality remains a separate formatter responsibility in Task 2.
- The parser is upstream recursive code; no additional recursion/resource
  budget or panic recovery is introduced in this task.
- No Task 2 work has begun. Independent review is still required.
