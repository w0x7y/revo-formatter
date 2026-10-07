# VS Code activation follow-up — 2026-10-06

Historical verification of the VS Code activation correction on 2026-10-06.
Counts, archive hash and host checks below describe that stage. Later work
fixed launcher signal handling and regenerated the local archive. Use the
[documentation index](../README.md) for current instructions and the
[later source/editor check](2026-10-06-editor-final-check.md) for subsequent
verification and its limits. The
[archived editor guide](https://github.com/w0x7y/revo-formatter/blob/a480fc40bb1fbbcf4379f509f3dad41837c90988/editors/README.md)
describes the packages before extraction.

## Reproduction and cause

The first VSIX installed successfully, but the user reported that Format Document
was unavailable. Their screenshot showed the existing document in Plain Text
mode. The installed package's source bytes and manifest matched the reviewed
checkout, and the executable setting pointed to the built formatter.
After reloading the window, the user confirmed Revo was available in Change
Language Mode.

An isolated native VS Code 1.140.0 probe exposed a second, independent failure:
`.rv` opened as `revo`, but showing the document and waiting for automatic
activation left the extension inactive. The native document-formatting provider
returned no edits for `let x=1`.

The manifest relied on implicit language activation. The current
[VS Code language contribution implementation](https://github.com/microsoft/vscode/blob/07f806f999227108933c2e30515b26eecc1fda74/src/vs/workbench/services/language/common/languageService.ts#L111-L117)
generates an implicit `onLanguage` event only for a language with both an ID and
a language configuration path. This package intentionally supplies only file
recognition, so its language declaration has no configuration path.

The correction declares `onLanguage:revo` explicitly. It retains the existing
language registration and provider without adding unrelated language behavior.
The package version becomes 0.1.1 so local testers can distinguish the corrected
VSIX from the original archive.

## Verification evidence

Commit `c8f86d7d` adds the explicit activation event, version 0.1.1 and a
dependency-free native regression runner under `editors/vscode/tests/host/`.
Run `npm run test:host` from the package directory with desktop VS Code available;
the package README describes prerequisites and executable overrides.

The native runner creates a temporary profile, empty extensions directory and
disposable workspace, without changing the user's settings or installed
extension. It opens and displays documents and waits for automatic activation;
it never explicitly calls the extension's `activate()` method.

| Check | Result |
| --- | --- |
| Native host before the manifest correction | Failed: Revo recognized, formatter inactive after five seconds |
| Native host after the correction | Passed on VS Code 1.140.0 for `.rv` and `.revo` |
| Unsaved source and native edits | `let x=2` became exactly `let x = 2\n`; disk source remained `let x=1` |
| Native idempotence | Second format returned no edits and retained exact buffer text |
| Native runner cleanup | Default cleanup passed; no probe host processes remained |
| VS Code unit/provider and process checks | 45 passed, zero failures |
| Shared editor runner | Passed: 27 Neovim, 45 VS Code and 15 Zed checks |
| VSIX package inspection | Ten entries; manifest, runtime, README and license match tested source bytes |

The corrected archive is `editors/vscode/revofmt-0.1.1.vsix`, with SHA-256
`b21ffbe650de1657e9748271d2d01a42f5f81fa9825f7651fe1ee2000a5f6d13`.
It remains a local, ignored build artifact. The runner executes the development
extension; archive inspection establishes that its shipped runtime and manifest
match the tested checkout.

Full formatter checks passed with the documented exact Zig 0.17.0 executable:
77 Rust tests, zero doctests, clippy with warnings denied, Rust formatting,
release build, Zig formatting, nine filtered Zig tests, and both checksum
manifests (23 vendor and 27 corpus files). The initial Cargo invocation omitted
`ZIG` and failed its toolchain precondition; rerunning with the documented
environment passed. Logs are local under `target/editor-verification/`.

The [fresh independent review](2026-10-06-vscode-activation-review.md) covered
`1e31cd51..3d48a57c` and approved the correction with no Critical, Important or
Minor findings. Final bookkeeping records that result without changing the
tested runtime or native runner.

## Coverage limits

The new native checks establish recognition, activation, provider execution,
edit application, unsaved buffer transport and idempotence on desktop Linux
x86_64 GNU with VS Code 1.140.0. They do not establish native behavior on the
minimum VS Code 1.85 release, remote hosts, Restricted Mode, undo, cancellation,
stale results, line-ending edge cases or save actions. Existing process/provider
tests retain their earlier coverage of those adapter concerns.

The user reported successful manual use of Neovim and Zed. That is user-reported
host evidence, not a new automated Zed test. Earlier
[editor verification](2026-10-06-editor-integrations.md) and
[whole-branch review](2026-10-06-editor-review.md) describe the original reviewed
stage and its narrower host-testing coverage.
