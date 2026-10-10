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
Linux x86_64 GNU. The [formatter README](../README.md#install) covers binary
installation and source builds.

The configuration-aware behavior described below is on each adapter
repository's `main` branch. New adapter releases are not yet published; until
they are, published adapter releases still send the pre-0.2.0 arguments,
`--indent-width N --line-width N -`, and do not read `revofmt.toml`. The Zed
extension on `main` downloads the v0.3.0 language server, which becomes
available when that release is published.

## CLI contract

Adapters send the complete current unsaved buffer to stdin in print mode. The
argument array always has this order, with `--stdin-filepath` present only for
real files:

```sh
revofmt --prefer-config --stdin-filepath /project/src/main.rv \
  --indent-width 2 --line-width 80 --indent-style space --max-blank-lines 1 -
```

For a buffer without a file path, the same array omits the `--stdin-filepath`
pair:

```sh
revofmt --prefer-config --indent-width 2 --line-width 80 \
  --indent-style space --max-blank-lines 1 -
```

They start an executable with an argument array and use stdout only after a
successful exit. They never invoke `--write` or format a saved copy instead of
the current buffer. Save formatting stays opt-in.

- **`--prefer-config`.** When a `revofmt.toml` applies to the buffer, it
  replaces all four layout settings from the editor. Keys the file omits use
  the formatter's built-in defaults, not the editor's values, so the editor
  agrees with a flagless `revofmt --check` in CI. When no file applies, the
  editor settings apply. The [configuration reference](formatter.md#configuration)
  describes discovery and [precedence](formatter.md#precedence).
- **`--stdin-filepath`.** It is the path from which the formatter looks for
  `revofmt.toml`, and it need not exist on disk, so an unsaved new file still
  finds its project. Adapters send it only for real files: Neovim for a buffer
  with an empty `buftype` and a nonempty name that is not a `scheme://` URI,
  expanded to an absolute path; VS Code for `file:` URIs, using `fsPath`; Zed
  for `file:` URIs, converted with `fileURLToPath`. Unnamed, scratch, virtual
  and remote documents omit it. Without a path the formatter performs no
  discovery and the editor settings apply.
- **Minimum version.** Adapters on this contract require revofmt 0.2.0 or
  later. An older CLI does not know `--prefer-config`, so it exits with code 2
  (`unrecognized option: --prefer-config`) and nothing is formatted. Each
  adapter reports the CLI's stderr, such as that message or a malformed
  `revofmt.toml`, as an error; Zed does so with `window/showMessage`.
- **Ranges.** The indent style is `space` or `tab`. Indentation is 1 through 8
  columns per level; a tab counts as that many columns when fitting lines. The
  line width is 20 through 240 columns and is a soft target. Blank lines are
  0 through 8, and 1 is the default. Each adapter validates its own settings
  against these ranges, and the CLI applies the same ranges to flags and
  `revofmt.toml`.

The CLI validates token/comment bytes, syntax equivalence modulo source
coordinates and idempotence. Its [input admission limits](verification/input-limits.md)
apply in every editor. [Exit codes](formatter.md#cli-usage) and stderr diagnostics
are part of the public integration boundary.

The adapter or native host owns process cancellation, deadlines, stale results
and edit application. Keep literal and comment bytes intact through the editor's
buffer representation. Neovim and VS Code reject unrepresentable output; Zed's
supported preservation scope is UTF-8 LF source because its native pipeline
normalizes endings. The owning guides describe exact host limits and the save
settings needed to prevent independent whitespace cleanup.

Revo syntax highlighting, completion and diagnostics remain separate from
formatting. Zed's `revofmt-lsp` extension supplies a formatting language server
for the existing Revo language without registering a language or grammar.
Install the [Revo language extension](https://github.com/w0x7y/revo-zed-extension)
alongside it, then apply the formatter settings from `revofmt-zed`.

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
suite requires Node.js and npm; Zed's CLI, language-server and launcher checks
require Python, Node.js and Rust.
The [VS Code development guide](https://github.com/w0x7y/revofmt-vscode/blob/main/docs/development.md)
separately documents native extension-host tests and VSIX packaging. Keep each
adapter's lifecycle and transport regressions in its own repository.
