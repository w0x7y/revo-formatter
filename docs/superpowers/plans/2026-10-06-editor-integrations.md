# Editor integrations implementation plan

Completed editor-integration plan from 2026-10-06. Interfaces and execution
steps below record the original stage; later work fixed VS Code activation and
Neovim timeout completion. Use the [documentation index](../../README.md) for
current instructions, the [editor guide](../../../editors/README.md) for delivered
packages, and the [later source/editor check](../../verification/2026-10-06-editor-final-check.md)
for later verification and its limits.

> Historical execution process: implementation used subagent-driven development
> task by task, with a fresh reviewer before each next implementation began.
> These instructions record the completed stage.

**Goal:** Locally installable formatting and file recognition for Neovim, VS Code, and Zed.

**Architecture:** The existing CLI owns formatting and preservation. Editor packages own byte transport and editor lifecycle, and live under `editors/` with their own tests and documentation.

**Tech stack:** Lua/Neovim, Node/VS Code extension API, declarative TOML/JSON for Zed, existing Rust/Zig CLI.

**Spec:** `docs/superpowers/specs/2026-10-06-editor-integrations-design.md`

## Global constraints

- Recognize `.rv` and `.revo` as Revo.
- Format whole unsaved buffers through an installed `revofmt` executable using stdin print mode, with an argument array and no shell.
- Apply stdout only on exit 0. Failed, canceled, timed-out, or stale operations leave the buffer untouched.
- Preserve opaque literals/comments and CLI output bytes; never globally normalize CRLF.
- Defaults: executable `revofmt`, indent width 2, line width 80. Bounds: indent 1..8, line 20..240.
- Format-on-save is opt-in. Do not change personal editor configuration or download a formatter binary.
- Only native Linux x86_64 GNU is verified for the binary; do not imply broader support.
- No formatter core, upstream pin, or vendor changes. Keep each package self-contained and avoid unrelated restructuring.
- Implementation order: Neovim, VS Code, Zed. Review each before starting the next; no registry publication or remote pushes.

## Task 1: Neovim plugin

**Files:** Create `editors/neovim/README.md`, `editors/neovim/LICENSE`, `editors/neovim/plugin/revofmt.lua`, `editors/neovim/lua/revofmt/init.lua`, focused transport/codec modules under `editors/neovim/lua/revofmt/`, and `editors/neovim/tests/` with a documented executable test runner.

**Interfaces:** Consumes CLI stdin/stdout. Produces `require('revofmt').setup({ executable, indent_width, line_width, timeout_ms, format_on_save })`, `.format({ bufnr, async })`, and `:RevoFormat`. Neovim >=0.10; defaults timeout 5000 ms, manual async, save disabled.

- [x] Write focused failing headless tests before implementation. Initial buffer `let x=1` must become `let x = 1\n` on serialization after format. Assert both suffixes select `revo`. Run with `nvim --headless -u NONE -i NONE -n -l editors/neovim/tests/run.lua` using `REVOFMT_BIN` to locate the real CLI.
- [x] Implement raw `vim.system` transport and codec. Reconstruct buffers using fileformat and endofline, decode without global CR stripping, and require exact re-encoding equality. Reject binary/unsupported encodings. Short-circuit empty buffers.
- [x] Add lifecycle behavior: buffer tick/options/generation guard, loaded/modifiable checks, bounded timeouts, stderr errors, one undoable minimal changed region, and view retention. Test edit during an async operation, overlapping operations, deletion, option changes, missing executable, syntax rejection, fake timeout, unchanged buffers, LF/CRLF/mixed literal endings, and idempotence using actual CLI output.
- [x] Add idempotent setup/file recognition/command registration and opt-in bounded synchronous `BufWritePre`. Test repeated setup and disabled/enabled save behavior.
- [x] Document local runtimepath/plugin-manager installation, options, command, Linux binary prerequisites, supported buffer formats, and verification. Keep package standalone.
- [x] Run the package tests, record results and known editor representation limitations, self-review, and commit only Task 1 files. Report to the assigned report file.

## Task 2: VS Code extension

**Files:** Create `editors/vscode/package.json`, a lockfile if dependencies are used, `editors/vscode/.vscodeignore`, `editors/vscode/LICENSE`, `editors/vscode/README.md`, focused source modules for process transport/provider/edit calculation, and `editors/vscode/tests/`.

**Interfaces:** Consumes CLI stdin/stdout independently of Task 1. Produces a Revo document formatting provider, language registration, and configurable `revofmt.executable`, `revofmt.indentWidth`, `revofmt.lineWidth`, `revofmt.timeoutMs`. Default timeout 5000 ms. Extension id `w0x7y.revofmt`; VS Code >=1.85. Node-based extension with no runtime dependency required.

- [x] Write failing tests through Node's built-in test runner. A formatting request for `let x=1` must return edits whose application equals `let x = 1\n`; real CLI transport must preserve CRLF bytes in opaque literals. Use `REVOFMT_BIN` for the executable. Test API registration and filename recognition through manifest/provider tests.
- [x] Implement direct executable spawning with bounded byte buffers, error handling, cancellation, and timeout. Limits: source/output 262144 bytes, stderr 65536 bytes. Reject invalid UTF-8 output. Never accept output on nonzero exit or after cancellation.
- [x] Implement provider wiring and a minimal contiguous replacement with correct UTF-16 positions. Snapshot document version and per-document generation; discard obsolete results. Check editor EOL normalization against exact output, rejecting unrepresentable mixed-ending output rather than changing literal bytes. Test unicode edit boundaries, empty source, unchanged output, syntax errors, settings validation, missing executable, output overflow, timeouts, cancellation, and stale/overlapping requests.
- [x] Register `.rv`/`.revo` as `revo`; use trusted-workspace execution only and mark executable settings appropriately. Keep format-on-save opt-in through documented `[revo]` editor settings.
- [x] Document local VSIX packaging/installation and development. Package with VSCE or an equivalent supported tool and inspect package runtime files/license. Run extension-host smoke tests if available; distinguish unit/provider tests from actual host tests.
- [x] Run package checks, self-review, and commit only Task 2 files. Report test/package evidence and any host-test limitations to the assigned report file.

## Task 3: Zed extension

**Files:** Create `editors/zed/extension.toml`, `editors/zed/languages/revo/config.toml`, `editors/zed/settings.json`, `editors/zed/README.md`, `editors/zed/LICENSE`, and `editors/zed/tests/` with a runnable test entry point.

**Interfaces:** Consumes CLI stdin/stdout. Produces language `Revo`, suffixes `rv` and `revo`, and mergeable settings for native external formatting. No Rust/WASM code or grammar is needed for this scope; validate current Zed compatibility from primary sources.

- [x] Write checks that initially fail because metadata/settings are absent. Parse TOML and JSON, assert both suffixes and language name, assert native external formatter command and exact layout arguments, and assert `format_on_save` is `off`, and assert `remove_trailing_whitespace_on_save` and `ensure_final_newline_on_save` are false.
- [x] Add declarative language metadata and external formatter settings using `revofmt --indent-width 2 --line-width 80 -`. Expose executable/layout customization through documented settings edits. Disable native trailing-whitespace and final-newline passes so the CLI owns whitespace edits even on failure.
- [x] Exercise the configured command with real CLI input/output and failure cases, including empty source, LF/CRLF/literals, invalid syntax, and idempotence. Zed handles lifecycle/stale result behavior; inspect its native formatter implementation and document actual smoke-test limits without pretending these process tests test a running editor.
- [x] Document installing as a dev extension, merging settings with existing Revo support, avoiding duplicate language definitions, and format-on-save opt-in.
- [x] Run checks, self-review, and commit only Task 3 files. Report outcomes and constraints to the assigned report file.

## Task 4: Repository navigation and final verification

**Files:** Create `editors/README.md`, `scripts/verify-editors`; update root `README.md`, `docs/README.md`, `docs/architecture.md`, `.gitignore`, and Cargo packaging exclusions if needed.

- [x] Add a concise root directory map and link the editor packages. Document shared CLI/error/EOL contracts once and keep editor-specific installation next to the package.
- [x] Add one editor verification command invoking each actual package runner with `REVOFMT_BIN`, and ignore generated dependencies/packages and implementation scratch files.
- [x] Run all editor checks and package validation. Run the full README verification: Rust all-target tests/doctests/clippy/fmt/release build, Zig fmt/filtered bridge tests, vendor and corpus checksum manifests from their directories.
- [x] Dispatch a fresh whole-branch reviewer over all implementation changes; fix substantive findings and re-review fixes. Record actual verification results and limitations in a dated report, link current guides, and commit the final navigation/documentation work.
