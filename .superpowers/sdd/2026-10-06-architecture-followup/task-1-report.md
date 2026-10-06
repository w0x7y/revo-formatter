# Task 1 implementation report

Implemented the native host launcher lifecycle fix from base `71745013` in
`/home/idan/.t3/worktrees/revo-formatter/t3code-e8f7f0a9`.

## Changes

`editors/vscode/tests/host/launch.js` now checks `child.signalCode` in the
existing result polling loop, immediately after the spawn-error check and before
the existing nonzero-exit check. A launcher terminated by SIGTERM reports
`VS Code launcher terminated by signal SIGTERM` and exits with code 1.

The runner continues polling after a launcher exits with code zero. Parsing a
result file and checking `result.success === true` remain the completion path.
The existing 60-second host deadline, detached-group cleanup, failure evidence
retention, successful temporary-directory removal and environment overrides are
unchanged. No private implementation exports or alternate runner interfaces
were added.

`editors/vscode/tests/host-launch.test.js` executes the actual launcher entry
point as a child process. Each case owns an isolated temporary directory,
redirects the runner's temporary data there with TMPDIR, and supplies a
controlled executable through VSCODE_BIN. REVOFMT_BIN is `/bin/true` because
these cases exercise lifecycle behavior rather than formatter transport. The
test helper bounds completion at three seconds, cleans up the controlled
detached group even if the runner is killed, and removes all fixture data.

The four cases cover:

- A launcher that terminates itself with SIGTERM, asserting prompt failure and
  a diagnostic identifying SIGTERM.
- A launcher that exits with code 23, asserting prompt failure and its exit-code
  diagnostic.
- A successful wrapper exit followed by a separate process publishing a host
  result after 500 milliseconds. The publisher writes a temporary file and
  renames it atomically; the runner must print that result, succeed, and remove
  its successful host directory.
- A successful wrapper exit without a result, asserting that the runner is
  still waiting at the test's one-second bound. This case also verifies the
  controlled wrapper started, so a stalled setup cannot satisfy the assertion.

These process cases skip Windows because they use POSIX executable fixtures and
detached process groups. The supported verified repository platform is native
Linux x86_64 GNU. The README now distinguishes these controlled process tests
from actual VS Code extension-host testing and documents prompt launcher
termination diagnostics.

## Red-green evidence

Before changing launch.js, ran from the repository root:

```sh
node --test --test-name-pattern='SIGTERM' editors/vscode/tests/host-launch.test.js
```

The sole test failed after 3010.874765 milliseconds. Its assertion was:

```text
host runner must report SIGTERM before the 3-second test deadline
true !== false
tests 1; pass 0; fail 1
```

The runner was still polling with no signal diagnostic when the fixture deadline
killed it. This establishes the original failure without waiting for the old
60-second timeout. The one-line signalCode check was then added.

Initial focused verification passed four tests with zero failures. The signal
case completed in 130.134316 milliseconds. After self-review added an explicit
wrapper-start assertion to the no-result case, final focused verification ran:

```sh
node --test editors/vscode/tests/host-launch.test.js
git diff --check
```

Exit code 0, four passed, zero failed, zero skipped. The signal case took
133.417554 milliseconds; total test duration was 1937.540444 milliseconds.
The diff whitespace check passed.

## Package and native verification

Ran `npm test` in `editors/vscode` with Node.js v26.10.0. The final package run
passed 49 tests, with zero failures, cancellations or skips, exit code 0. This
includes the four new launcher process cases and all existing provider, edit
and real-CLI transport cases. The CLI used by those existing tests is the
default `../../target/debug/revofmt`.

Ran the existing native process interface in `editors/vscode`:

```sh
REVOFMT_BIN=/home/idan/.t3/worktrees/revo-formatter/t3code-e8f7f0a9/target/debug/revofmt VSCODE_BIN=/usr/bin/code npm run test:host
```

Exit code 0. The host published:

```json
{
  "vscodeVersion": "1.140.0",
  "files": [
    { "suffix": "rv", "language": "revo", "output": "let x = 2\n", "unsaved": true },
    { "suffix": "revo", "language": "revo", "output": "let x = 2\n", "unsaved": true }
  ],
  "extensionVersion": "0.1.1",
  "activeBeforeFormatting": true,
  "success": true
}
```

The existing suite also checks repeated formatting for idempotence. The native
run used the installed desktop VS Code with DISPLAY=:0 and the existing debug
formatter. It retained no successful host directory. No user configuration,
dependency versions, formatter sources or vendored upstream files changed.
No Rust/Zig suites, package publication, push or dependency downloads were run;
broad repository verification belongs to the coordinator's final check.

## Exploration and self-review

Read the task brief, applicable root AGENTS.md, editor README, root build and
domain guidance, current architecture, scan evidence, and the testing and
verification skill instructions. No module boundaries changed.

The scout already established caller closure: package.json invokes this script
through `npm run test:host`, with no indexed incoming symbol callers. I did not
repeat that closure scan. The launcher file API call reported an additional
approximately 923 tokens saved. Graft freshness reported no deep graph manifest
and an in-sync wiring graph. The first source read preceded the successful
Graft file API request, so 923 is Graft's estimate, not a measured reduction of
that earlier read. The initial file API call used the wrong argument key and
was corrected to `file` without changing repository data.

Self-review confirmed that signal and nonzero exit checks stay ahead of result
reading, preserving the established failure precedence; successful wrapper exit
still requires a host result; failure evidence remains available; successful
temporary data is removed; and the new tests invoke the process entry point
without loading private modules. The delayed publisher stays in the wrapper's
detached group so the existing runner cleanup can terminate it. The test
helper has fallback cleanup for its own timeout case.

Only the launcher, its new process regression file, package README and this
report are task files. No subagents, helpers or reviewers were spawned. A fresh
independent review is still required by the brief and must be arranged by the
coordinator; this report is a self-review, not that independent review.

## Remaining limits

No implementation concerns found in self-review. Windows process regressions
are skipped and Windows host behavior was not verified. The tests establish
prompt signal failure and host-result completion through the current process
interface; they do not expand the native suite's documented coverage of remote
workspaces, Restricted Mode, undo, cancellation or save actions. Independent
review and broad final verification remain coordinator responsibilities.
