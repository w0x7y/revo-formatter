# revofmt for Neovim

A standalone Lua plugin for Neovim **0.10 or newer**. It formats the complete
current unsaved buffer through an installed `revofmt`, recognizes `.rv` and
`.revo`, and registers `:RevoFormat`. There are no Lua dependencies.

For normal plugin-manager installation, use the standalone
[revofmt.nvim repository](https://github.com/w0x7y/revofmt.nvim). Its lazy.nvim
install hook or `:RevoFmtInstall` downloads a pinned, checksum-verified formatter,
and `:checkhealth revofmt` diagnoses setup. The downloadable binary currently
supports Linux x86_64 GNU with glibc >=2.34. That plugin needs no formatter
checkout or local Rust/Zig toolchain.

This directory retains the self-contained local adapter and its regression
checks. Its commands and executable settings are documented below; managed
installation belongs to the standalone plugin.

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

For lazy.nvim, save this as a module in the `lua/plugins/` directory that your
configuration imports, for example `lua/plugins/revofmt.lua`. Point `dir` to
the Neovim package inside the checkout:

```lua
return {
  {
    dir = '/absolute/path/to/revo-formatter/editors/neovim',
    name = 'revofmt',
    config = function()
      require('revofmt').setup({
        executable = '/absolute/path/to/revo-formatter/target/release/revofmt',
      })
    end,
  },
}
```

The package can also be copied intact into a native package directory such as
`~/.local/share/nvim/site/pack/revofmt/start/revofmt/`. Its `plugin/` and `lua/`
directories must be directly inside that directory. Default command registration
works without a setup call. Repeated `setup` calls replace configuration and save
hooks without duplicating them. Loading the plugin preserves setup already made
in your init file. Existing nonempty filetype assignments are respected.

Open a `.rv` or `.revo` file and check `:set filetype?`; it should report `revo`.
If another plugin has assigned a different filetype, select it explicitly with
`:set filetype=revo` before relying on this plugin's save hook.

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
Errors are also reported through `vim.notify`. Formatter errors include bounded
CLI stderr. A new request supersedes the previous request for that buffer.

Set `format_on_save = true` to install a bounded synchronous `BufWritePre` hook
for buffers with filetype `revo`. The same save writes the formatting result. A
formatter failure leaves the buffer untouched and the save writes the user's
source. Manual formatting is asynchronous by default and does not write files.

Other save hooks can modify the source independently of this plugin. Avoid
whitespace cleanup hooks for Revo when they would change opaque literal or
comment contents.

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
unloading, option changes, superseded requests, timeouts, signal termination,
subprocess errors, and unrepresentable output leave the buffer untouched.
Changed lines are applied as one minimal contiguous replacement, preserving
native undo and displayed views
where possible; cursor positions are clamped naturally if a line becomes shorter.

The plugin rejects source above 262,144 bytes before launching the formatter.
Subprocess stdout has the same cap; stderr is capped at 65,536 bytes. The
transport owns public `vim.uv` pipes and a timer. Excess output, timeouts, and
cancellation terminate the direct child and close those pipes. A wrapper
descendant retaining stdout or stderr cannot keep the request pending past its
deadline while the editor event loop runs. Late output is discarded;
independently running descendants remain the wrapper's responsibility.

Process execution uses an executable plus arguments directly. Paths containing
spaces are supported. Shell expressions, `~` and environment-variable expansion
are not interpreted. Choose an executable you trust; the plugin has no workspace
trust gate and runs the command with Neovim's privileges.

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
timeouts (including wrappers with inherited pipes), deadline settlement and
handle cleanup, excessive output, and output representation rejection.

The [latest verification record](../../docs/verification/2026-10-06-documentation-handoff.md)
reports all 32 checks passing on Neovim 0.12.5 with the release CLI. The public
APIs were reviewed for Neovim 0.10 compatibility, but that minimum version was
not executed. POSIX subprocess fixtures do not establish Windows support.

Neovim API references: [vim.uv](https://neovim.io/doc/user/luvref/),
[buffer APIs](https://neovim.io/doc/user/api/#api-buffer), and
[fileformat](https://neovim.io/doc/user/options/#'fileformat').
