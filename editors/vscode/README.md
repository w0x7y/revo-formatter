# Revo Formatter for VS Code

This local extension recognizes `.rv` and `.revo` files as Revo and formats the
complete current buffer, including unsaved changes, with an installed `revofmt`.
Its extension ID is `w0x7y.revofmt`. It requires VS Code 1.85 or later and a trusted
workspace. File recognition remains available in Restricted Mode.

The formatter binary is built and verified only on native Linux x86_64 GNU.
This extension does not bundle or download a formatter. Build the CLI from the
repository root with exact Zig 0.17.0 and Rust, following the root README, then
put `revofmt` on PATH or configure its absolute path.

## Package and install locally

From this directory, with Node.js 22 or later and npm:

```sh
npm ci
npm run package
code --install-extension revofmt-0.1.1.vsix
```

You can also use **Extensions: Install from VSIX…** from VS Code's Command
Palette. No Marketplace publication is required. Packaging uses the pinned
development-only `@vscode/vsce`; the installed extension has no runtime npm
dependencies. The VSIX includes its MIT license.

## Format a document

Open a `.rv` or `.revo` document and run **Format Document**. If another formatter
is installed, choose **Format Document With… → Revo Formatter**. Language
recognition can also be selected manually for an untitled document.

The extension runs on the workspace extension host. In Remote SSH, containers or
WSL, install the extension and formatter in that environment and configure a
path that exists there. Browser-only and virtual workspaces are unsupported.

The settings are:

| Setting | Default | Accepted values |
| --- | --- | --- |
| `revofmt.executable` | `revofmt` | Executable name on PATH or executable path |
| `revofmt.indentWidth` | `2` | Integer 1–8 |
| `revofmt.lineWidth` | `80` | Integer 20–240; a soft target |
| `revofmt.timeoutMs` | `5000` | Integer 1–2147483647 milliseconds |

Indent width uses the formatter default, independently of `editor.tabSize`.
The executable setting is machine-overridable and restricted in untrusted
workspaces. Review a workspace's executable setting before trusting it. The
executable is spawned directly with an argument array; shell quoting, `~`,
environment-variable expansion and additional arguments are not interpreted.
For example, configure `"revofmt.executable": "/home/me/bin/revofmt"`.

## Opt in to format on save

The extension does not change editor settings. Add this language override to
your user or workspace `settings.json` if you want save formatting:

```json
{
  "[revo]": {
    "editor.defaultFormatter": "w0x7y.revofmt",
    "editor.formatOnSave": true,
    "editor.formatOnSaveMode": "file",
    "files.trimTrailingWhitespace": false,
    "files.insertFinalNewline": false,
    "files.trimFinalNewlines": false
  }
}
```

The `files` overrides keep independent save-time whitespace passes from changing
opaque multiline literals or comments, including when formatting fails. Review
other extensions' save actions separately if they also modify Revo documents.

## Preservation and failures

The CLI owns syntax validation, exact token/comment and syntax-tree preservation,
and idempotence. This extension transmits UTF-8 source on stdin and only uses
stdout from a successful subprocess. Source and stdout are limited to 262144
bytes; stderr is limited to 65536 bytes. Lone UTF-16 surrogates and invalid UTF-8
stdout are rejected. Cancellation, timeout, nonzero exit, signal termination,
document changes and superseded requests leave the buffer untouched. Errors
include bounded CLI stderr when available.

Successful formatting returns one small contiguous edit with UTF-16 positions.
VS Code normalizes inserted line breaks to the document's LF or CRLF setting.
The extension verifies that applying this normalization to the proposed edit
would still produce the exact CLI output; otherwise it reports an error and
returns no edit. It never globally rewrites CRLF. Mixed line endings in untouched
parts of the buffer are retained when the minimal edit is representable.

These guarantees concern the current editor buffer. VS Code may already have
normalized mixed raw-file endings when loading a document, before the extension
can inspect it. The adapter cannot restore bytes lost during loading or by
independent save actions. Ordinary UTF-8 LF and CRLF buffers are supported.

## Development and verification

Build the CLI first, then run the dependency-free Node test suite:

```sh
npm test
REVOFMT_BIN=/absolute/path/to/revofmt npm test
```

The default test executable is `../../target/debug/revofmt`. Tests exercise real
CLI transport and a small VS Code API test double for provider registration,
edit application, workspace trust, document lifecycle and cancellation. Controlled
subprocesses cover malformed output, process limits and termination. These are
unit/provider and CLI integration tests; they are not actual extension-host
tests.

An installed desktop VS Code and a working display are required for the separate
native extension-host regression test. It downloads no tools or dependencies:

```sh
npm run test:host
REVOFMT_BIN=/absolute/path/to/revofmt VSCODE_BIN=/absolute/path/to/code npm run test:host
```

The host runner defaults to `../../target/release/revofmt` and `code` on PATH.
It opens a separate window with temporary user settings, an empty extensions
directory and a disposable trusted workspace. It checks `.rv` and `.revo`
recognition, automatic activation after displaying a Revo document, native
provider edits, exact output from an unsaved buffer and idempotence. It never
calls the extension's activation function directly. The extension declares
`onLanguage:revo` explicitly because its metadata-only language contribution
does not receive VS Code's implicit activation event. See the
[VS Code language contribution implementation](https://github.com/microsoft/vscode/blob/07f806f999227108933c2e30515b26eecc1fda74/src/vs/workbench/services/language/common/languageService.ts#L111-L117).

The runner waits up to 60 seconds for a host result, even when the `code` launcher
returns early. It closes only its isolated host and removes successful temporary
data. Failures retain logs and results; set `REVOFMT_HOST_KEEP=1` to retain a
successful run too. The native test passed on Linux x86_64 GNU with VS Code
1.140.0. It does not establish host behavior for remote workspaces, Restricted
Mode, undo, cancellation, stale results, line-ending edge cases or save actions.

For a manual extension-host smoke test on a machine with VS Code installed:

```sh
code --new-window --extensionDevelopmentPath="$(pwd)"
```

In that window, trust a disposable workspace, configure a built CLI, and create
both `example.rv` and `example.revo` containing `let x=1`. Confirm their language
is Revo, change the unsaved buffer, and run **Format Document**. Check the final
newline, undo, CRLF multiline literals, syntax errors and opt-in save formatting.
In an untrusted workspace, formatting must report the trust requirement without
executing the binary. These manual checks cover behavior beyond the automated
host regression; package tests and VSIX inspection alone do not establish it.

API and packaging references: [document formatting providers](https://code.visualstudio.com/api/language-extensions/programmatic-language-features#format-source-code-in-an-editor),
[workspace trust](https://code.visualstudio.com/api/extension-guides/workspace-trust),
[remote extension hosts](https://code.visualstudio.com/api/advanced-topics/remote-extensions),
and [VSIX packaging](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).
