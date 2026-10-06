# Architecture follow-up verification, 2026-10-06

This follow-up starts at `aea81c06` and implements the two confirmed architecture
scan findings. Final production sources are at `ae82b578`. The
[architecture guide](../architecture.md) describes their current ownership.
Earlier dated reports retain the counts and coverage of their recorded stages.

## Changes and regression evidence

`0f18e168` fixes native VS Code test-launcher signal handling in its existing
polling loop. A signal-terminated launcher now fails promptly and names the
signal. A zero wrapper exit still requires a successful host result; cleanup
and the 60-second deadline remain unchanged. Four real process-entry regressions
cover SIGTERM, nonzero exit, delayed atomic host-result publication and zero exit
without a result. The SIGTERM regression failed at its three-second bound before
the fix, then passed in about 133 ms. No extension runtime change was needed.

`15a5690a` moves block-envelope ownership to the source collector. One table
records child-first completed block ends and finalizes existing block, statement
and body hints. Rust consumes `block` hints and pairs punctuation lexically.
Both raw keyword-balancing stacks were removed. Literal-output public formatter
tests first reproduced lost indentation from `foo.end`, `let do=1` and
`let end=1`; all three failed before the change and passed afterward. The shared
test helper checks the real token/AST preservation interface and idempotence.
Metadata tests check exact envelopes, including nested final blocks and short
declaration wrappers. Admission, public interfaces and metadata shape remain
unchanged.

The first independent source review found Important issue I1: suppressing every
hint beneath generated pipe wrappers also hid actual source blocks needed by
Rust. Long block bodies aligned with their opener and nested block indentation
flattened, although preservation still passed. The coordinator ruled that
proven source-block facts must survive generated ancestry while synthetic
statement/header hints and opaque descendants remain excluded. If that
provenance distinction is wrong, synthetic metadata could leak and require
rework. Clarification `3fbbb9ba` and fix `ae82b578` implement that boundary.
Two literal-output regressions plus an exact whole-region test failed before
fix1 and passed afterward. They cover both pipe sides, long/nested/function
blocks, synthetic-hint exclusion and opaque interiors.

The scan's deletion test justified retaining `LayoutIndex`, `Doc`, editor
transport/codecs/edits and provider/settings seams. Deleting them would spread
existing complexity among callers. CLI batch prevalidation and the preservation
oracle needed no restructuring. A shared editor runtime or new harness layer
offered no demonstrated leverage. The two fixes improve locality within existing
owners rather than adding another module interface.

## Verification

All source checks below ran after the last production-source change, using
native Linux x86_64 GNU and exact Zig 0.17.0. Cargo source commands set
`ZIG=/home/idan/.local/share/revo/toolchain/zig-x86_64-linux-0.17.0/zig`.
The editor/native checks then used the rebuilt absolute release executable.
Documentation-only completion reused that unchanged-source evidence.

| Check | Final result |
| --- | --- |
| `cargo test --all-targets` | 88 passed: 71 library, one binary, 16 CLI |
| `cargo test --doc` | Passed; zero doctests |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo fmt --check` | Passed |
| Zig formatting and local filtered tests | Passed; nine tests |
| `cargo build --release` | Passed |
| `scripts/verify-editors` | Passed: 27 Neovim, 49 VS Code, 15 Zed |
| `npm run test:host` in `editors/vscode` | Passed with `/usr/bin/code`, VS Code 1.140.0 |
| Vendor/corpus `sha256sum --check SHA256SUMS` | Passed from their directories; 23/27 entries |
| VSIX packaging and archive/source comparison | Passed; ten entries, 10.54 KB, runtime/manifest/README/license match |
| Cargo package list | Passed; editors, their dependencies and scratch files excluded |
| Offline Cargo packaged build | Passed; 119 files, extracted crate built successfully |
| Changed-document relative links and `git diff --check` | Passed; five Markdown files, 69 relative links; no whitespace errors |

The Zig commands were `zig fmt --check bridge.zig bridge/*.zig` and
`zig test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter
indexed --test-filter 'input limits:' --cache-dir target/zig-test-cache`.
These filters select local bridge/index/admission tests, not upstream runtime
tests. The Cargo package build uses the same absolute `ZIG` with
`cargo package --offline --allow-dirty`; dirty allowance covers guide edits.
Builds downloaded no tools or source.

The Rust suite includes all 120 corpus/option combinations, reviewed goldens,
actual preservation/idempotence checks, bounded-stack/admission probes and CLI
batch prevalidation. Corpus sources, goldens, manifests, vendor source, licenses,
pins and budgets are unchanged. Repeated nested metadata fixtures at
200/400/800 units measured about 5.97/11.19/21.66 ms in one debug run. This sample
is consistent with roughly linear scaling, not a benchmark guarantee. The
trusted owning-module fixtures exceed public token admission while keeping
nesting bounded; separate public tests exercise admitted input on a 2 MiB stack.
Earlier architecture performance measurements were not rerun.

Native VS Code checks establish `.rv`/`.revo` recognition, automatic activation
of extension 0.1.1, whole unsaved-buffer formatting, native edit application and
idempotence. They do not establish minimum VS Code 1.85, remote hosts, Restricted
Mode, native undo/cancellation/stale results, line-ending edge cases or save
actions. Controlled launcher process regressions use POSIX fixtures and skip
Windows. No Windows host or other formatter platform was verified. Neovim
package tests are headless; Zed checks are metadata/real-CLI tests, not a new
native-host run.

The rebuilt local v0.1.1 VSIX has SHA-256
`e780e3535c824b74412693e1293c16666cf77d4ae4210943715203a97e0f61b0`.
It remains an ignored local artifact. No publication, push, user editor
configuration change or dependency download occurred.

## Reviews and remaining gate

| Scope | Actual review result |
| --- | --- |
| Task 1, `71745013..0f18e168` | Approved; no findings |
| Initial Task 2, `0f18e168..15a5690a` | Needs fixes; Important I1 |
| Task 2 fix1, `15a5690a..ae82b578` | I1 addressed; no new breakage |
| Task 3 documentation/packaging | Fresh task review pending |
| Whole follow-up, `aea81c06..HEAD` | Fresh final review pending |

The coordinator will append the actual Task 3 and whole-review verdicts after
those reviews. The plan's final-review checkbox remains open. This report does
not claim whole-change approval.

Full local command/output evidence is under `target/architecture-review/`.
Final source logs use `task-2-fix1-` prefixes; final editor/native logs are
`final-fix1-editors.log` and `final-fix1-native-host.log`. Task 3 packaging and
document logs use `task-3-` prefixes. The ignored
`.superpowers/sdd/2026-10-06-architecture-followup/` ledger contains task reports,
independent review reports and coordinator checks. The results, I1 correction
and verification limits above are retained here without requiring those local
scratch files.
