# Editor final check, 2026-10-06

This records the completed review before the repository documentation refresh.
Use the [documentation index](../README.md) for current guides and the
[documentation handoff](2026-10-06-documentation-handoff.md) for later packaging
and documentation checks. Counts, hashes and limits below belong to this stage.

Reviewed scope: the complete feature since `main` merge base
`e048b7d598608aa702bf38713334fb87bb5ae159` through
`2bb293393b2c4a74fad3b8d154359fd5ab274cc3`, plus the working-tree repairs
recorded below. The initial tree and index were clean. The review ended with
unstaged repairs and created no commits or published packages. Earlier dated reports
retain the counts and verification limits of their recorded stages.

## Confirmed findings and repairs

- **P1: source block admission.** The public formatter admitted 300 nested
  ordinary blocks but aborted on a 2 MiB debug Rust thread during recursive
  document construction. Lexer-only admission now charges possible block
  introducers against the shared layout budget before recursive work, including
  decoded bodies and contextual keyword names. Mixed recursive forms require
  combined charges. The [input policy](input-limits.md) defines the final weights.
- **P2: Neovim inherited pipes.** A formatter wrapper could exit while a descendant
  retained stdout/stderr. The old synchronous wait returned no completed result,
  causing an exception that aborted `BufWritePre`; async timeout completion also
  waited for inherited EOF. Transport now owns public `vim.uv` pipes/timers and
  settles once at its deadline. Failed formatting leaves source unchanged and
  save continues. Five regressions cover synchronous/manual-async deadlines,
  single completion, cancellation/handle cleanup and actual save behavior.
- **Documentation accuracy.** The root README no longer promises that every
  statement newline survives formatting: short block statements inside pipe
  expressions can share a line. Exact token/comment and syntax preservation
  contracts remain unchanged.

The original core failure was reproduced in isolated debug subprocesses before
repair; release accepted that same input. The Neovim regression suite failed
five cases against the original transport and passed all 32 after repair.
No corpus source, reviewed golden, vendor byte, pin, license, numerical budget
maximum or dependency version is changed.

## Verification

All source checks used native Linux x86_64 GNU and exact Zig 0.17.0, selected
through its absolute `ZIG` path. Final editor and native host checks used the
rebuilt absolute release formatter. Rust debug/release suites and clippy ran
after the final test cleanup; unchanged production-source Zig/build evidence
was retained. Documentation completion does not change those source results.

| Check | Result |
| --- | --- |
| `cargo test --all-targets` | 89 passed: 72 library, one binary, 16 CLI |
| `cargo test --release --all-targets` | Same 89 passed |
| `cargo test --doc` | Passed; zero doctests |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo fmt --check` | Passed |
| Zig formatting and filtered local tests | Passed; ten tests |
| `cargo build --release` | Passed |
| `scripts/verify-editors` | Passed: 32 Neovim, 49 VS Code, 15 Zed |
| Native VS Code `npm run test:host` | Passed on 1.140.0 for both suffixes |
| Vendor/corpus checksum manifests | Passed from their directories; 23/27 entries |
| Existing VSIX source/archive comparison | Passed; ten entries, version 0.1.1 |
| Offline Cargo package and extracted build | Passed; 120 files, editors/dependencies/scratch excluded |
| `cargo audit` and `npm audit --json` | Zero vulnerability advisories |
| Changed Markdown relative targets | 18 documents, 89 targets, zero missing files |
| Working-tree/index/untracked text whitespace | Passed; index unchanged |

The admission regression exercises 44 admitted and 16 rejected scenarios on
an explicit 2 MiB thread, with actual token/AST preservation and idempotence
checks for admitted results. Existing accepted prefix/operator boundaries and
all 120 corpus/option combinations remain intact. It covers plain and contextual
blocks, block/operator/pipe/function/control/unary mixtures in both orders,
punctuation nesting, opaque contents, decoded bodies and globally combined
fragments. The new CLI rejection fixtures exercise no-output/no-write behavior.
These cases are bounded evidence rather than a proof for every grammar shape.

Local Zig checks use `zig fmt --check bridge.zig bridge/*.zig` and
`zig test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter
indexed --test-filter 'input limits:' --cache-dir target/zig-test-cache`.
Filters select local bridge/index/admission tests, not upstream runtime tests.
The existing VSIX runtime, manifest, README and license match source bytes;
no VS Code source changed during this check, so the archive was not regenerated.
Its SHA-256 is
`e780e3535c824b74412693e1293c16666cf77d4ae4210943715203a97e0f61b0`.

Fresh independent fix reviews found no actionable defects. Neovim review added
native stdin/output-bound/cancellation/cleanup probes. Admission review added
18 isolated debug/release subprocess probes of block/postfix/match/return and
punctuation intersections on 2 MiB threads; supported cases completed and
adjacent receiver limits rejected cleanly. Both original findings are addressed
within the documented scope. Final documentation and diff checks are recorded
in the verification table.

Offline packaging used `cargo package --offline --allow-dirty` with exact `ZIG`.
The extracted crate compiled successfully without downloading tools or source;
its repaired core sources match the checkout. Subsequent changes complete the
verification prose only. No package was published.

## Review coverage and limits

Fresh reviewers inspected all 52 original changed paths across core, Neovim/Zed,
VS Code, package/configuration and documentation scopes. Repository-wide native
security review included unchanged CLI file handling, FFI, source/candidate
admission, preservation, recursive layout, build and pinned frontend entry points.
Editor reviews traced executable launch, trust, buffer guards, byte conversion,
resource limits and failure cleanup. Root review inspected dependency locks,
packaging exclusions, configuration inventory and tracked secret indicators.
No CI, server, authentication, container or deployment surface was found.

`cargo audit` and `npm audit --json` returned zero vulnerability advisories. Cargo
used the RustSec database updated 2026-10-03. No production npm dependencies are
shipped; its 134 locked dependencies are development-only. A limited tracked-text
scan found no private-key, common token or authenticated-URL indicators.
Gitleaks and Semgrep were unavailable. These checks supplement code review;
they do not prove the absence of vulnerabilities.

Native execution is limited to Linux x86_64 GNU with exact Zig 0.17.0. Neovim
checks use 0.12.5; minimum 0.10 has public API source review but no execution here.
VS Code native coverage uses 1.140.0: recognition, automatic activation, unsaved
buffers, native edits and idempotence. Minimum 1.85, remote hosts, Restricted
Mode, native undo/cancellation/stale-result/EOL/save scenarios remain unverified.
Zed checks are declarative metadata and actual CLI execution, without a new
native-host run; its preservation scope remains LF source. User-reported working
hosts are separate evidence. POSIX subprocess fixtures do not establish Windows
support. Neovim kills the direct formatter child; wrappers own their descendants.

This is bounded regression and manual security review, not exhaustive fuzzing or
an audit of every unused vendored runtime. CLI atomic replacement remains per
file rather than per batch and is not intended as privileged processing of
attacker-owned mutable directories. No live exploitation, credential change,
publication or personal editor configuration change was performed.

Complete local review reports, probe programs and command logs are under the
ignored `target/final-check/` directory. The original feature documents are
historical evidence, not instructions for the current repairs.
