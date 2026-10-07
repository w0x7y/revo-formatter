# Editor integrations

Editor adapters are maintained separately from the formatter:

| Editor | Repository | Formatting entry point |
| --- | --- | --- |
| Neovim >=0.10 | [revofmt.nvim](https://github.com/w0x7y/revofmt.nvim) | `:RevoFormat` |
| VS Code >=1.85 | [revofmt-vscode](https://github.com/w0x7y/revofmt-vscode) | Format Document |
| Zed | [revofmt-zed](https://github.com/w0x7y/revofmt-zed) | `editor: format` |

Use each repository's README for installation and settings. Each package owns
its source, metadata, license, tests and packaging. This repository owns the
Rust library, CLI and pinned frontend. It does not contain copies of the adapters
or require editor tools for formatter builds and tests.

The Neovim plugin can download and verify a pinned formatter binary. VS Code and
Zed use a separately installed CLI. The verified formatter platform is native
Linux x86_64 GNU. The [formatter README](../README.md#get) covers binary
installation and source builds.

## CLI contract

Adapters send the complete current unsaved buffer to stdin in print mode:

```sh
revofmt --indent-width 2 --line-width 80 -
```

They start an executable with an argument array and use stdout only after a
successful exit. They never invoke `--write` or format a saved copy instead of
the current buffer. Save formatting stays opt-in.

Indentation accepts 1 through 8 spaces; line width accepts 20 through 240 columns
and is a soft target. The CLI validates token/comment bytes, syntax equivalence
modulo source coordinates and idempotence. Its [input admission limits](verification/input-limits.md)
apply in every editor. [Exit codes](formatter.md#cli-usage) and stderr diagnostics
are part of the public integration boundary.

The adapter or native host owns process cancellation, deadlines, stale results
and edit application. Keep literal and comment bytes intact through the editor's
buffer representation. Neovim and VS Code reject unrepresentable output; Zed's
supported preservation scope is UTF-8 LF source because its native pipeline
normalizes endings. The owning guides describe exact host limits and the save
settings needed to prevent independent whitespace cleanup.

Revo syntax highlighting, completion and language-server support remain separate.
If another extension already registers Revo in Zed, use only the formatter
settings from `revofmt-zed` to avoid duplicate language registration.

## Verification

First rebuild the formatter using the [main verification commands](../README.md#development-and-verification).
When a CLI change affects an adapter, run that repository's documented checks
against the rebuilt executable. With the repositories cloned under `~/GitRepo`,
these commands run from the formatter repository root:

```sh
REVOFMT_BIN="$PWD/target/release/revofmt" ~/GitRepo/revofmt.nvim/scripts/verify
REVOFMT_BIN="$PWD/target/release/revofmt" ~/GitRepo/revofmt-vscode/scripts/verify
REVOFMT_BIN="$PWD/target/release/revofmt" ~/GitRepo/revofmt-zed/scripts/verify
```

These are optional downstream checks, not a combined test suite in this
repository. Neovim requires its supported editor and Python; VS Code's process
suite requires Node.js and npm; Zed's metadata and CLI checks require Python.
The [VS Code development guide](https://github.com/w0x7y/revofmt-vscode/blob/main/docs/development.md)
separately documents native extension-host tests and VSIX packaging. Keep each
adapter's lifecycle and transport regressions in its own repository.
