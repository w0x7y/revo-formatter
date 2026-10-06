# Editor integrations

These packages add whole-buffer formatting and Revo file recognition. Install
the package for your editor, then select an installed `revofmt` executable.
Both `.rv` and `.revo` files use the Revo language. Format-on-save is opt-in.

| Editor | Package and installation | Formatting entry point |
| --- | --- | --- |
| Neovim >=0.10 | [Lua plugin](neovim/README.md) | `:RevoFormat` |
| VS Code >=1.85 | [VSIX extension](vscode/README.md) | Format Document |
| Zed | [Dev extension and settings](zed/README.md) | Format Buffer |

Each directory owns its source, metadata, tests, license, and README. There
is no shared editor runtime or language server. Syntax highlighting,
completion, and diagnostics remain the responsibility of existing language
tooling. The packages have not been published to editor registries.

## Install the formatter

Build the CLI using the [root build instructions](../README.md#build-from-source).
The verified binary platform is native Linux x86_64 GNU. Editor APIs may run
on other systems, but this repository does not provide or verify formatter
binaries for them.

Use the absolute path to `target/release/revofmt`, or install the built binary
in a directory on the editor's PATH. Editors started from the desktop may
have a different PATH from your terminal. The integrations do not download
the binary or require Zig at runtime.

## Shared formatting contract

Every integration sends the current unsaved buffer on stdin in print mode:

```sh
revofmt --indent-width 2 --line-width 80 -
```

The executable path and layout arguments are configurable. Indentation
accepts 1 through 8 spaces; line width accepts 20 through 240 columns and is
a soft target. The integrations use stdout only when the command succeeds.
Syntax, validation, and process failures leave the buffer unchanged and
report an error. They never invoke `--write` or format a saved copy in place.

The CLI validates source preservation and idempotence. Editor transports must
preserve its output too: CRLF inside a multiline string or comment is opaque
source, not a line ending to normalize. Neovim and VS Code reject output that
cannot be represented safely in the current buffer. Their guides describe
editor-specific limits. Zed delegates execution and result application to its
native external formatter.

Zed's native buffer pipeline normalizes line endings, so its end-to-end
preservation scope is LF source. Read its guide before formatting CRLF or
mixed-ending files.

## Development

Keep changes and regression tests inside the owning editor package. The Rust
formatter tests stay in their existing locations. The architecture guide
describes the [package boundaries](../docs/architecture.md#editor-packages).

After building the CLI, with the tools listed in the root verification guide:

```sh
scripts/verify-editors
# Or use an existing debug build:
REVOFMT_BIN="$PWD/target/debug/revofmt" scripts/verify-editors
```

This runs the headless Neovim, VS Code process/provider, and Zed configuration
checks. See each package's README for individual commands and editor-host
smoke tests. Generated dependencies and VSIX archives remain outside version
control and the Cargo source package.
