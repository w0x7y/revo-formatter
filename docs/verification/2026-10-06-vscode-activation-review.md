# VS Code activation fix review

Historical review of the VS Code activation correction on 2026-10-06. Line
references, package hash and checks below apply to the reviewed range. The
[documentation index](../README.md) links later launcher and editor reviews;
the [editor guide](../../editors/README.md) describes current packages, and the
[later source/editor check](2026-10-06-editor-final-check.md) records subsequent
verification and its limits.

Reviewed range: `1e31cd512fc353d69b8342425898cc3b36a51bee..3d48a57c24ee6f6c13578a6d8a8467a9e03dbf1f`, 2026-10-06.

Ready: yes. No actionable findings.

## Spec compliance

The change addresses the reproduced activation failure with explicit `onLanguage:revo`, without changing formatter runtime modules, language behavior, dependencies, upstream code or personal editor configuration. Version 0.1.1 is consistent across the package, lockfile and package command. The package still recognizes both suffixes and uses the existing native formatting provider and complete unsaved-buffer stdin transport.

The pinned [VS Code language contribution source](https://github.com/microsoft/vscode/blob/07f806f999227108933c2e30515b26eecc1fda74/src/vs/workbench/services/language/common/languageService.ts#L111-L117) confirms that implicit language activation requires an ID and a configuration path. The metadata-only Revo declaration lacks that path, so explicit activation is the appropriate correction.

The native regression is meaningful. The red run recognized Revo but failed the activation assertion after displaying the document and waiting five seconds. The separate earlier native probe also records no formatter edits. The green run uses automatic activation, with no direct `activate()` call, and checks both suffixes through the native command and native edit application. Its literal expected output distinguishes the dirty buffer's `x=2` from the backing file's `x=1`; disk bytes remain unchanged, and a second format returns no edits while retaining exact buffer text.

Critical: none. Important: none. Minor: none.

## Code quality and project standards

The small manifest fix preserves existing provider, subprocess, admission and preservation contracts. The new runner keeps launcher responsibilities separate from assertions executed inside VS Code. It needs no new dependencies and is excluded from both the ordinary unit-test glob and the VSIX.

Launcher isolation is suitable for the tested desktop Linux host. A unique temporary profile, empty extensions directory, disposable workspace and isolated XDG configuration avoid personal settings and installed extensions. Clearing the inherited CLI IPC hook prevents forwarding into the user's running window. Formatter configuration names the admitted absolute executable, and launch arguments use direct argv invocation. Atomic result publication avoids partial JSON; polling requires the host result even if the CLI wrapper exits early. Cleanup targets the detached process group of this isolated launch. The supplied default-cleanup run succeeded, and no matching probe processes remain. Failure evidence is retained.

Independent ZIP inspection found exactly ten entries. The manifest, README, license and all five runtime modules match source bytes. The archive declares version 0.1.1 and explicit activation, excludes tests and dependencies, and has the recorded SHA-256 `b21ffbe650de1657e9748271d2d01a42f5f81fa9825f7651fe1ee2000a5f6d13`.

Critical: none. Important: none. Minor: none.

## Evidence and limits

Read contributor instructions, build/domain/architecture guides, the exact prepared diff and commit range, existing provider/transport/configuration code and tests, package exclusions, local fix report, native red/green/cleanup evidence, package log and full-verification logs. The logs support 45 VS Code checks, the shared 27/45/15 editor counts, 77 Rust tests, zero doctests, nine Zig tests, clippy, formatting, release build and both checksum manifests. The documented initial missing-`ZIG` failure and successful rerun are accurate.

Documentation distinguishes the user's initial Plain Text/reload observation from the independently reproduced activation failure. It also distinguishes development-extension execution from archive byte inspection. Native coverage is explicitly limited to VS Code 1.140.0 on desktop Linux x86_64 GNU; minimum-version, remote, Restricted Mode and other listed host scenarios remain unverified. Those disclosed limits do not invalidate this narrowly scoped fix. The review-pending line is truthful at the reviewed commit and can now be updated by the coordinator.

No unchanged passing suite was rerun, and no uncovered risk warranted a new execution probe. No tracked files, index, HEAD, installed extension or personal editor settings were changed. This ignored report is the only written file. No subagents were dispatched.
