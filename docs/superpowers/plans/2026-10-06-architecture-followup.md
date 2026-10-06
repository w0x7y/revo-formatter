# Architecture Follow-up Implementation Plan

> **For agentic workers:** Use subagent-driven-development to implement the tasks with fresh reviews. The user explicitly authorized implementation without a visual report or further candidate approval.

**Goal:** Fix the two confirmed architecture-scan findings while preserving existing formatter and editor contracts.

**Architecture:** Actual parsed blocks own their lexical envelopes in the source collector; the Rust layout index consumes those facts. The native host launcher retains one local lifecycle rule and is tested through its process interface.

**Tech stack:** Existing Rust/Zig formatter, dependency-free Node tests and native VS Code host.

**Spec:** `docs/superpowers/specs/2026-10-06-architecture-followup-design.md`

## Global constraints

- Preserve token/comment tape, complete AST modulo coordinates, opaque source and idempotence.
- Preserve parser-sensitive adjacency, existing fitting rules, CLI modes/exit codes and complete write-batch prevalidation.
- Keep current editor setup/settings/commands and independently installable packages.
- Keep existing admission before recursive work and candidates; no budget, vendor, toolchain or upstream pin changes.
- Verify native Linux x86_64 GNU with exact Zig 0.17.0.
- No speculative seams, shared editor runtime, exported private test hooks, personal editor configuration changes, publication or push.

## Task 1: Native host launcher lifecycle

**Files:** `editors/vscode/tests/host/launch.js`, a process regression file under `editors/vscode/tests/`, and package README if testing descriptions change.

**Interface:** Existing `npm run test:host`, with `VSCODE_BIN` and `REVOFMT_BIN` overrides. Tests invoke the existing script as a child process, using isolated temporary directories and a controlled launcher.

- [x] Add a process regression whose launcher terminates itself with SIGTERM. Bound the test well below the old 60-second deadline and assert failure identifies the signal.
- [x] Cover nonzero exit and successful early wrapper exit followed by atomic host result publication. Successful wrapper exit alone must never count as a host pass.
- [x] Run the focused tests to record the signal case failing before the fix.
- [x] Observe `child.signalCode` alongside `child.exitCode` in the existing polling loop, preserving result-file completion and detached-group cleanup.
- [x] Run focused tests, package tests and native host verification; commit only this task's files and report exact results. Obtain a fresh independent review.

## Task 2: Source-backed block envelopes

**Files:** `bridge/source.zig`, `src/layout_index.rs`, `src/tests/formatting.rs`, and owning-module metadata tests where necessary. Update current architecture documentation through Task 3.

**Interfaces:** Preserve the bridge's current `block` source-region metadata shape and `LayoutIndex::new`/scope interfaces. The source collector supplies complete block lexical envelopes; Rust consumes them for block pairing. Punctuation pairs stay lexical.

- [x] Add focused public formatter regressions with literal expected output, actual preservation and idempotence. Start with these source/output pairs:

```text
fn f() do\nfoo.end\nbar()\nend
fn f() do\n  foo.end\n  bar()\nend\n

do\nlet do=1\nfoo()\nend
do\n  let do = 1\n  foo()\nend\n

do\nlet end=1\nfoo()\nend
do\n  let end = 1\n  foo()\nend\n
```

- [x] Cover comments between a dot and keyword field, keyword names in parameters/declarations/labels, nested final blocks and declaration spans that omit descendants. Use real parser acceptance to select valid fixtures; preserve compact header and parser-sensitive adjacency expectations.
- [x] Record focused failures before modifying ownership.
- [x] Resolve concrete block envelopes child-first in the existing source collector, preserving generated/opaque exclusion and admission. Reuse one table of complete block envelopes for block, statement and header source hints. Remove raw keyword pairing as semantic authority.
- [x] Retain proven source-block facts beneath generated wrappers, including multi-statement and nested blocks in lowered pipe expressions. Synthetic statement/header hints and opaque descendants remain excluded; test both facts and resulting block indentation.
- [x] Populate Rust block pairs from emitted `block` hints, retaining lexical punctuation pairing and range-bounded scope behavior. Remove obsolete keyword-pairing logic rather than layering a second path.
- [x] Run focused formatting/metadata tests, full formatting/corpus/admission checks and exact Zig tests. Investigate any golden difference rather than blindly regenerating expected output. Commit only this task's files, report results, and obtain a fresh independent review.

## Task 3: Current guides and final verification

**Files:** `docs/architecture.md`, `docs/README.md`, root README counts if changed, and `docs/verification/2026-10-06-architecture-followup.md`.

**Interface:** Current contributor guides describe final ownership and real verification evidence. Historical plans/reports retain their recorded stage.

- [x] Explain block lexical-envelope ownership and the launcher lifecycle fix; record scan areas where the deletion test did not justify restructuring.
- [x] Run full README Rust/Zig/checksum/editor verification, native VS Code checks, Cargo packaging checks and diff/link checks on the completed changes.
- [x] Dispatch a fresh final reviewer over `aea81c06..HEAD`, fix substantive findings and re-review fixes. Record actual results and limits, mark this plan complete and commit documentation.
