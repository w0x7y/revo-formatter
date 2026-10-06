# revofmt for Neovim

A standalone Lua plugin for Neovim **0.10 or newer**. It formats the complete
current unsaved buffer through an installed `revofmt`, recognizes `.rv` and
`.revo`, and registers `:RevoFormat`. There are no Lua dependencies.

Build or install the CLI first using the [repository instructions](../../README.md#build-from-source).
The verified binary platform is native Linux x86_64 GNU. Running the compiled
binary does not require Zig; building it requires the repository's exact pinned
Zig toolchain. This plugin does not download binaries or provide syntax
highlighting, completion, diagnostics, or a language server.

## Install from a local checkout

Put this in your `init.lua`, replacing the absolute path:

```lua
vim.opt.runtimepath:prepend('/absolute/path/to/revo-formatter/editors/neovim')
require('revofmt').setup({
  executable = '/absolute/path/to/revo-formatter/target/release/revofmt',
})
```

For a plugin manager, use the Neovim package directory inside the monorepo,
not the repository root. For example, with lazy.nvim and a local checkout:

```lua
{
  dir = '/absolute/path/to/revo-formatter/editors/neovim',
  name = 'revofmt',
  config = function()
    require('revofmt').setup({ executable = '/absolute/path/to/revofmt' })
  end,
}
```

The package can also be copied intact into a native package directory such as
`~/.local/share/nvim/site/pack/revofmt/start/revofmt/`. Its `plugin/` and `lua/`
directories must be directly inside that directory. Default command registration
works without a setup call. Repeated `setup` calls replace configuration and save
hooks without duplicating them. Loading the plugin preserves setup already made
in your init file. Existing nonempty filetype assignments are respected.

## Configuration and use

These are the defaults. Each setup call starts from these defaults:

```lua
require('revofmt').setup({
  executable = 'revofmt', -- executable path or command on PATH; no shell syntax
  indent_width = 2,       -- integer 1..8
  line_width = 80,        -- integer 20..240; soft width target
  timeout_ms = 5000,      -- positive integer, maximum 2147483647
  format_on_save = false,
})
```

Run `:RevoFormat` for asynchronous formatting. The Lua interface also accepts
an explicit buffer and synchronous mode:

```lua
require('revofmt').format() -- current buffer, asynchronous
require('revofmt').format({ bufnr = 0, async = false })
```

`format` returns `true` when an asynchronous request starts or synchronous
formatting succeeds. Immediate or synchronous failure returns `false, message`.
Asynchronous errors are reported through `vim.notify`. Formatter errors include
CLI stderr. A new request supersedes the previous request for that buffer.

Set `format_on_save = true` to install a bounded synchronous `BufWritePre` hook
for buffers with filetype `revo`. The same save writes the formatting result. A
formatter failure leaves the buffer untouched and the save writes the user's
source. Manual formatting is asynchronous by default and does not write files.

## Bytes, buffers, and lifecycle

Supported buffers use UTF-8 (`fileencoding` empty or `utf-8`), `fileformat=unix`
or `dos`, `nobinary`, and `nobomb`. Unsupported representations are refused.
NUL-bearing formatter output is also refused. The plugin encodes buffer lines
with the current `fileformat` and `endofline`, sends raw bytes on stdin, and
requires an exact byte round trip before accepting stdout. For DOS output it
removes exactly the CR belonging to each CRLF delimiter. Other CR bytes,
including those inside opaque literals and comments, remain intact. Output that
the chosen fileformat cannot represent exactly is rejected.

An empty one-line buffer stays untouched. Neovim's public line representation
cannot reliably distinguish an empty buffer from a source containing one
newline; both are handled as canonical empty buffers. Formatting updates the
`endofline` option when the CLI adds a final newline. Neovim's native undo
history records line edits but does not undo option changes such as `endofline`.
An EOL-only change still marks the buffer modified. Your `fixendofline` setting
continues to govern subsequent native file writes.

Before applying a result, the plugin checks the request generation, changedtick,
loaded/modifiable state, and serialization options. Edits, buffer deletion or
unloading, option changes, superseded requests, timeouts, signal termination, subprocess errors, and
unrepresentable output leave the buffer untouched. Changed lines are applied as
one minimal contiguous replacement, preserving native undo and displayed views
where possible; cursor positions are clamped naturally if a line becomes shorter.

Subprocess stdout is capped at the CLI's 262,144-byte admission limit and stderr
at 65,536 bytes. Excess output and timeouts terminate the child process. Process
execution uses an executable plus arguments directly; paths containing spaces
are supported and shell expressions are never evaluated.

## Verification

From the repository root, with a built CLI and Neovim on PATH:

```sh
REVOFMT_BIN="$PWD/target/debug/revofmt" \
  nvim --headless -u NONE -i NONE -n -l editors/neovim/tests/run.lua
```

`tests/run.lua` is the executable headless runner and uses no personal editor
configuration. If `REVOFMT_BIN` is unset it uses `target/debug/revofmt`. Controlled
subprocess tests require Python 3. The suite exercises actual Neovim buffers and
the real CLI for formatting, idempotence, LF/CRLF and mixed literal bytes, undo,
view retention, syntax failures, save behavior, and startup/file recognition.
Controlled subprocess fixtures exercise races, option changes, unloading,
timeouts, excessive output, and output representation rejection.

Neovim API references: [vim.system](https://neovim.io/doc/user/lua/#vim.system()),
[buffer APIs](https://neovim.io/doc/user/api/#api-buffer), and
[fileformat](https://neovim.io/doc/user/options/#'fileformat').
