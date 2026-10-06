# Editor integrations

These packages add whole-buffer formatting and Revo file recognition. Install
the package for your editor, then select an installed `revofmt` executable.
Both `.rv` and `.revo` files use the Revo language. Format-on-save is opt-in.
Build the CLI first; installing an editor package does not install the formatter.

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

The executable path and layout arguments are configurable. Each integration
starts the executable directly with an argument array, without a shell.
Indentation accepts 1 through 8 spaces; line width accepts 20 through 240 columns and is
a soft target. The integrations use stdout only when the command succeeds.
With the documented settings, syntax, validation, and process failures leave
the buffer unchanged and report an error. They never invoke `--write` or format
a saved copy in place. Independent editor save actions can still change source;
the VS Code and Zed guides disable whitespace passes that could alter literals.

The CLI validates source preservation and idempotence. Editor transports must
preserve its output too: CRLF inside a multiline string or comment is opaque
source, not a line ending to normalize. Neovim and VS Code reject output that
cannot be represented safely in the current buffer. Their guides describe
editor-specific limits. Zed delegates execution and result application to its
native external formatter. Neovim and VS Code default to a five-second process
deadline; Zed has no timeout setting supplied by this package.

Zed's native buffer pipeline normalizes line endings, so its end-to-end
preservation scope is LF source. Read its guide before formatting CRLF or
mixed-ending files.

The CLI's [input admission limits](../docs/verification/input-limits.md) apply
in every editor. Neovim and VS Code also check the 262,144-byte source limit
before launching the process and bound stdout and stderr. The formatter's
token and recursive-form limits are checked by the CLI, not by editor parsers.

## Development

Keep changes and regression tests inside the owning editor package. The Rust
formatter tests stay in their existing locations. The architecture guide
describes the [package boundaries](../docs/architecture.md#editor-packages).

After building the CLI, use Neovim >=0.10, Node.js >=20, npm and Python >=3.11:

```sh
scripts/verify-editors
# Or use an existing debug build:
REVOFMT_BIN="$PWD/target/debug/revofmt" scripts/verify-editors
```

This runs the headless Neovim, VS Code process/provider, and Zed configuration
checks. See each package's README for individual commands and editor-host
smoke tests. Generated dependencies and VSIX archives remain outside version
control and the Cargo source package.

The [latest verification record](../docs/verification/2026-10-06-documentation-handoff.md)
reports 32 Neovim, 49 VS Code and 15 Zed checks. Neovim ran on 0.12.5; the
separate native VS Code host check ran on 1.140.0. Minimum editor versions and
remote hosts have not been exercised. Zed's automated checks cover metadata and
CLI subprocesses, rather than a native editor session. User-reported successful
use of all three integrations is separate from that automated coverage.
