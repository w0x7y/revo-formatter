# Editor integrations

Approved design for the completed editor-integration stage of 2026-10-06.
Package boundaries and proposed transport below record that stage. Later work
fixed VS Code activation and replaced Neovim's original transport. Use the
[documentation index](../../README.md) for current instructions, the
[editor guide](../../../editors/README.md) for delivered packages, and the
[later source/editor check](../../verification/2026-10-06-editor-final-check.md) for later
verification and its limits.

The approved implementation order was Neovim, VS Code, Zed, with an independent
review before each next implementation began. Registry publication was outside
that stage.

## Repository layout

Keep the Rust library and CLI at the repository root. `src/`, `bridge/`,
`vendor/`, and the formatter's existing `tests/` keep their responsibilities.
Place independently installable editor packages under `editors/neovim/`,
`editors/vscode/`, and `editors/zed/`. Each package owns its source, tests,
metadata, and installation README. `editors/README.md` explains their common
CLI contract and links to package instructions. `scripts/verify-editors`
provides one command for editor checks, without mixing Node dependencies into
the Rust crate. Keep generated packages and dependencies out of git and the
Cargo source package. Do not move unrelated existing code or historical docs.

## Shared behavior

- Recognize `.rv` and `.revo` as Revo.
- Format the complete current unsaved buffer via an installed `revofmt`.
- Invoke an executable and argument array directly, without a shell. Send
  UTF-8 source on stdin using print mode (`-`); apply stdout only on exit 0.
- Expose executable path, indent width (1..8), and line width (20..240).
  Default to `revofmt` on PATH, indent width 2, and line width 80.
- Save formatting is opt-in. Do not edit users' editor configuration.
- Preserve CLI output bytes through the editor's buffer representation.
  Never globally normalize CRLF because literal/comment contents are opaque.
  Reject an output representation that would alter those contents.
- A failed, canceled, timed-out, or stale operation leaves the buffer untouched.
  Report useful errors, including CLI stderr. No automatic binary downloads.
- Keep whole-buffer formatting as the only formatting operation. Syntax
  highlighting, completion, diagnostics providers, and language servers are
  outside v1. Existing language tooling should continue to work.
- The formatter binary is verified only on native Linux x86_64 GNU. Editor
  packages must document that limit without implying other binary support.

## Neovim

Standalone dependency-free Lua plugin for Neovim >=0.10. Register the file
type and `:RevoFormat`; expose `require('revofmt').setup` and `.format`.
Keep transport/codec separate from buffer lifecycle. Use raw `vim.system`
stdin/stdout, scheduling its callback onto the editor loop. Snapshot changed
tick, generation, fileformat and endofline. Apply only to the same still-loaded,
modifiable buffer and original state. Support ordinary UTF-8 Unix/DOS buffers;
reject unsupported encodings or binary mode. Decode with an exact byte
round-trip check, preserving carriage returns that belong to opaque source.
Bounded synchronous `BufWritePre` formatting is opt-in so the same save writes
the result. Preserve undo and view where possible. Empty buffers stay empty.
Document the editor's inability to distinguish an empty buffer from a single
newline through its public line representation. Provide a local runtimepath
installation and monorepo plugin-manager instructions.

## VS Code

Provide language registration plus a native document formatting provider.
Keep process handling, text-edit calculation, and VS Code wiring in separate
focused modules. Spawn the configured CLI with byte-preserving transport,
bounded output, cancellation, and a timeout. Guard against document version
changes and superseded requests. Return a small contiguous edit; verify that
VS Code EOL normalization would not alter the formatted bytes before returning
it. Respect workspace trust and explain executable-path settings. Use VS Code's
existing format-on-save setting, documented as an opt-in Revo override.
Provide a locally packageable VSIX with its license and correct runtime files.

## Zed

Use a declarative language extension registering Revo and both suffixes.
Current Zed source supports an optional grammar; omit it because v1 does not
provide syntax highlighting. Supply mergeable native external-formatter
settings with `revofmt`, its layout arguments, and save formatting disabled.
Disable native `remove_trailing_whitespace_on_save` and
`ensure_final_newline_on_save` for Revo: native whitespace passes run before
the external command and could change opaque bytes even on CLI failure.
Zed owns process execution and applying native formatting results. Test metadata,
settings, and the exact stdin/stdout contract with the real CLI. Document any
editor-level verification unavailable in this environment honestly. Zed's
native buffer pipeline normalizes line endings; full raw-file preservation is
limited to LF sources. Document CRLF/mixed-ending limitations instead of
claiming that command transport checks establish host-level preservation.

## Verification

Headless Neovim tests exercise actual buffers and the real CLI, including EOL
and opaque-literal transport, idempotence, undo, errors, save behavior, and
stale operations. Controlled subprocess fixtures cover timeouts and races.
VS Code tests exercise process handling, provider wiring, cancellation, stale
results, and edit application, with real CLI cases for syntax/byte preservation.
Verify a real VSIX package; use an extension-host smoke test if the environment
supports it. Zed checks parse manifests/settings and exercise their command.
Run the README's full Rust/Zig verification and checksum manifests as well.
