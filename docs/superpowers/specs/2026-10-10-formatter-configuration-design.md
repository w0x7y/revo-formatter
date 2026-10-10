# Formatter configuration

Design agreed on 2026-10-10 for three customization options: tab indentation,
a configurable blank-line limit, and a discovered project configuration file.
It also covers the adapter changes that let Neovim, VS Code and Zed honor that
file.

## Goals

- A project can commit `revofmt.toml`. `revofmt --check` in CI and all three
  editor adapters then produce identical output for files under it.
- Indentation can use tabs.
- The number of consecutive blank lines kept is configurable.
- Preservation, idempotence, the four-pass fixed-point bound, input admission
  and exit codes are unchanged. Default options produce the same bytes as the
  starting commit `f9c95329`.

## Non-goals

- Options that change tokens: trailing commas, quote style, parentheses, import
  order or comment wrapping. The token tape must stay byte-identical.
- A user-level or global config, merging nested configs, a `.revofmt.toml`
  alias, `.editorconfig` support and `--config PATH`.
- Reading editor-native indentation settings (`expandtab`, `tabSize`,
  `insertSpaces`, `hard_tabs`, LSP `FormattingOptions`).
- A minimum blank-line count, or alignment that mixes tabs and spaces.
- Measuring tabs inside opaque tokens. `unicode-width` counts them as zero
  columns today, and that limitation stays.
- Directory recursion and `--diff` output.
- Runtime CLI version probing in adapters.

## Library options

`src/lib.rs` adds a public enum and two public fields:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndentStyle {
    #[default]
    Space,
    Tab,
}

pub struct FormatOptions {
    pub indent_width: usize,      // 1..=8, default 2
    pub line_width: usize,        // 20..=240, default 80
    pub indent_style: IndentStyle, // default Space
    pub max_blank_lines: usize,   // 0..=8, default 1
}
```

`FormatOptions::validate` adds the `max_blank_lines` range to its single error
message. Adding public fields breaks callers that construct the struct with a
literal, so the crate version moves from 0.1.2 to 0.2.0. The library stays free
of filesystem and configuration code. `IndentStyle` does not derive serde
traits; the configuration module maps strings to it.

## Layout

### Tabs

Every leading indentation the renderer writes is a whole number of levels. No
layout aligns to an arbitrary column. In tab mode `src/document.rs` writes
`depth` tab characters where space mode writes `depth * indent_width` spaces.
Column tracking and fit decisions count each tab as `indent_width` display
columns, as Prettier's `tabWidth` does. `--indent-style tab --indent-width 4`
therefore breaks lines exactly where four-space indentation would.

Operator continuations (`Fill` at `depth + 1`), match arms, block bodies and
conservative layout use the same depth arithmetic, so leading indentation never
mixes tabs and spaces. `render` receives an indentation unit and that unit's
display width instead of a bare `indent_width`, keeping `document.rs`
independent of `FormatOptions`.

Opaque tokens keep their bytes. Revo's multiline-string dedent counts space and
tab alike inside the token, and comment and literal interiors are never
reindented.

### Blank lines

`Builder::breaks` in `src/layout.rs` changes from `clamp(1, 2)` to
`clamp(1, max_blank_lines + 1)`. Like the current one-line cap, it applies to
preferred and conservative layout. The comment on `layout` that describes
conservative mode as "capping blank lines at one" is updated.

The option only removes blank lines; it never adds them. Output therefore never
contains more blank lines than the source, and admission accounting does not
change. A limit of 0 removes every blank line. The pinned parser compares line
numbers (`Parser.zig` statement-start checks) but never counts blank lines, and
doc comments attach as attribute statements. Every candidate still passes the
preservation check before it is returned.

## Configuration file

### Format

The file is named `revofmt.toml` and parsed with the `toml` crate, a new
dependency of the binary only. All keys are optional:

```toml
indent_width = 4
line_width = 100
indent_style = "tab"      # "space" or "tab"
max_blank_lines = 2
```

Key names match the `FormatOptions` fields. The following are errors that name
the configuration path:

- an unknown key (deserialization denies unknown fields)
- a value of the wrong type, including negative integers
- an `indent_style` other than `space` or `tab`
- a value outside the library ranges, checked after resolution

A file using keys from a newer release fails on an older CLI.

Reading is bounded to 64 KiB plus one byte, matching the CLI's bounded source
reads, and the contents must be UTF-8. A `revofmt.toml` entry that exists but is
not a readable regular file is an error. A symbolic link to a regular file is
followed. Deeply nested TOML values must produce an error rather than exhaust
the stack on a 2 MiB thread. The `toml` 1.x parser applies a recursion guard
unless its `unbounded` feature is enabled, so the dependency must not enable
that feature. A unit test confirms the error.

### Discovery

For each input, discovery starts at the canonical form of the deepest existing
ancestor of the input's parent directory and walks its ancestors to the
filesystem root. The first `revofmt.toml` found applies to that input. A
`--stdin-filepath` naming directories that do not exist is therefore judged by
the real location of the nearest directory that does: missing trailing
directories are skipped to the deepest existing ancestor, whose real location is
used. A dangling symbolic link, or a `..` after a missing component, is still
walked as written.

Results, including "no configuration" and errors, are cached per directory for
one CLI run, so a configuration file is read and parsed at most once. In
`--check` and `--write`, each file uses its own nearest configuration, so one
batch can span subprojects with different settings.

### Precedence

| Situation | Sources, highest first |
| --- | --- |
| Default | flags, then `revofmt.toml`, then built-in defaults, per key |
| `--prefer-config` and a configuration applies | `revofmt.toml`, then built-in defaults; all layout flags ignored |
| `--prefer-config` and no configuration applies | flags, then built-in defaults |
| `--no-config` | flags, then built-in defaults; `--prefer-config` has no effect |

Layout flags are `--indent-width`, `--line-width`, `--indent-style` and
`--max-blank-lines`. Under `--prefer-config`, keys absent from the file use
built-in defaults rather than flag values. This makes an editor using
`--prefer-config` agree with a flagless `revofmt --check` run in CI.

Precedence is resolved per input. In a batch, files with a configuration and
files without one can resolve differently.

## CLI

New options in `--help`:

```
  --indent-style S    Indent with space or tab (default space)
  --max-blank-lines N Consecutive blank lines kept (0 through 8; default 1)
  --stdin-filepath P  Find revofmt.toml from P when reading stdin
  --prefer-config     Ignore layout flags when revofmt.toml applies
  --no-config         Do not search for revofmt.toml
```

The `--indent-width` help text notes that in tab mode the value is the tab's
display width.

Flag values are validated during argument parsing with the existing usage
errors, even when `--prefer-config` later ignores them.

Stdin input performs no discovery unless `--stdin-filepath` is given. That path
need not exist; a relative path resolves against the current directory; and it
is used only for discovery. Diagnostics still name the input `stdin`. Passing
`--stdin-filepath` without stdin input, including with file paths, is a usage
error with exit code 2.

Configuration errors are per-input errors with exit code 2. A diagnostic names
the input and the configuration path. In `--check`, the CLI reports the error
and continues checking other inputs, and the error takes precedence over exit
code 1. In `--write`, configuration resolution is part of batch prevalidation:
a malformed configuration for any input prevents every write. Existing exit
codes, print-mode behavior and atomic replacement are unchanged.

## Module ownership

| Module | Change |
| --- | --- |
| `src/lib.rs` | `IndentStyle`, new fields, defaults and validation |
| `src/document.rs` | Indentation unit and display width in `render` |
| `src/layout.rs` | Blank-line clamp from options; indentation passed to `render` |
| `src/config.rs` (new, binary) | Discovery, per-directory cache, bounded reading, TOML parsing, resolution into `FormatOptions` |
| `src/cli.rs` | New flags, stdin path rules, per-input options from `config` |
| `src/main.rs` | Declares `mod config` |

`cli.rs` keeps argument parsing, modes, diagnostics and writes. It does not
parse configuration files.

## Editor adapters

### Shared contract

Every adapter spawns:

```sh
revofmt --prefer-config [--stdin-filepath P] --indent-width N --line-width N --indent-style S --max-blank-lines N -
```

- New adapter settings: indent style (`space` or `tab`, default `space`) and
  maximum blank lines (0 through 8, default 1). Validation matches the CLI
  ranges.
- `--stdin-filepath` is sent only for real files:
  - Neovim: `buftype` is empty and `nvim_buf_get_name` is nonempty, expanded to
    an absolute path. Buffer names that start with a URI scheme and `://` are
    not sent.
  - VS Code: `document.uri.scheme === 'file'`, using `fsPath`.
  - Zed: a `file:` document URI, converted with `fileURLToPath`.
- Untitled and non-file buffers omit `--stdin-filepath` and use adapter
  settings.
- Adapters require revofmt 0.2.0 or later. An older CLI rejects
  `--prefer-config` with exit code 2, and the adapter reports that error. Each
  README documents the minimum version and that a project `revofmt.toml`
  overrides adapter layout settings for files under it.
- Adapters continue to send the whole unsaved buffer on stdin, apply only
  successful current output and never invoke `--write`.

### Neovim (`revofmt.nvim`)

- Add `indent_style` and `max_blank_lines` to `setup` defaults and validation
  in `lua/revofmt/init.lua`. Pass the buffer path to the transport.
- Build the new argv in `lua/revofmt/transport.lua`.
- Pin `lua/revofmt/release.lua` to v0.2.0 with its published checksum.
- Update the exact argv in `tests/fixture.py`. Add real-CLI tests in
  `tests/run.lua` for a temporary `revofmt.toml`, the new settings and an
  unnamed buffer.

### VS Code (`revofmt-vscode`)

- Contribute `revofmt.indentStyle` (enum) and `revofmt.maxBlankLines` (integer
  0 through 8) in `package.json` with `resource` scope.
- Read and validate them in `src/settings.js`. Pass the file path from
  `src/provider.js` to `src/transport.js`.
- Update argv and settings tests, and move the CI CLI pin in
  `.github/workflows/test.yml` to v0.2.0.

### Zed (`revofmt-zed`)

- Add `indentStyle` and `maxBlankLines` to `src/options.rs` validation,
  `server/main.cjs` and the shipped `settings.json`.
- Pass the document path from `server/main.cjs` to `server/formatter.cjs`.
- When the CLI exits with code 2, send `window/showMessage` (type Error) with
  its stderr, while still returning no edits. Today stderr is discarded, so a
  malformed `revofmt.toml` would fail silently.
- Update lifecycle, protocol and Rust option tests, and move the CI CLI pin to
  v0.2.0.

Each adapter bumps its own version under its existing release process.

## Testing

### Formatter

`src/tests/formatting.rs` gains literal-output cases, each with preservation
and idempotence checks:

- Tabs: block bodies, operator continuations, match arms, conservative fallback
  inside a block, and the fit boundary at tab width 4 compared with four
  spaces.
- Blank lines: limits 0 and 2 between statements, inside blocks, in lists,
  after comments, and with CRLF source.
- Default options produce the same output as the starting commit for existing
  cases.

`src/tests/corpus.rs`:

- `combinations()` covers widths 24, 80 and 120, indent widths 2 and 4, both
  indent styles and blank-line limits 0, 1 and 2: 36 option sets over 20
  fixtures, 720 results (from 120). Reviewed expected outputs remain keyed to
  space indentation with limit 1.
- `stress_options()` covers widths 20, 24, 40, 80, 120 and 240, indent widths
  1, 2, 4 and 8, both styles and limits 0, 1 and 2. That is 144 option sets and,
  with LF and CRLF, 288 combinations per source. Across the 269 stress sources
  this gives 77,472 combinations (from 12,912). Measured on 2026-10-10, the
  corpus module's eight tests take 0.77 s in release and 2.64 s in debug, so
  the sixfold increase is affordable.

`src/config.rs` unit tests cover parsing every key, unknown keys, wrong types,
negative and out-of-range values, nearest-configuration-wins, no
configuration, the size bound, invalid UTF-8, a non-regular `revofmt.toml`, a
symbolic link to a regular file, and deeply nested values on a 2 MiB thread.

`tests/cli.rs` process tests cover:

- each precedence row, including a partial configuration under
  `--prefer-config`
- `--stdin-filepath`, including a nonexistent path and a relative path
- `--stdin-filepath` with file inputs as a usage error
- a batch spanning two subprojects with different configurations
- a malformed configuration preventing every `--write` replacement
- `--check` continuing past a configuration error and exiting with code 2
- invalid flag values rejected under `--prefer-config`
- the help text

`src/tests/input_limits.rs` needs no new cases: no option adds a recursive
path, and candidate admission is unchanged.

### Adapters

Each adapter's `scripts/verify` runs against the rebuilt 0.2.0 CLI with
`REVOFMT_BIN`, as described in `docs/editors.md`.

## Documentation

- `README.md`: new options, a configuration example and precedence summary.
- `docs/formatter.md`: CLI usage, a configuration section, library example with
  the new fields, and removal of "There is no directory discovery or
  configuration file support in this version."
- `docs/editors.md`: the new argv, `--prefer-config` and the 0.2.0 minimum.
- `docs/architecture.md`: `src/config.rs` ownership and per-input resolution.
- `CONTEXT.md`: a "Project configuration" glossary entry.
- `docs/README.md`: link this design and its plan in the plans table, and the
  verification record when complete.
- Adapter READMEs: new settings, minimum version and configuration precedence.

## Rollout

1. Implement and verify the formatter changes. Run every command in the README's
   development and verification section, then bump the version to 0.2.0.
2. Publish the v0.2.0 release with its binary and `SHA256SUMS`. Tagging and
   publishing need explicit approval at that time.
3. Update Neovim, VS Code and Zed against the published binary, run each
   adapter's `scripts/verify`, and record the results in a dated verification
   document in this repository.
