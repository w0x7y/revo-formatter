# revofmt for Zed

This package recognizes `.rv` and `.revo` as `Revo` and supplies settings for
Zed's native external formatter. It formats the complete current buffer through
an installed `revofmt`, including unsaved edits. Save formatting is opt-in.

**Use this integration for UTF-8 LF sources.** Zed's native buffer and diff
pipeline normalizes line endings. It cannot guarantee raw CRLF or mixed-ending
file preservation, including CRLF inside opaque literals or comments. The CLI
itself preserves those bytes, but it receives Zed's buffer text here. Use the CLI
directly when preserving the original CRLF or mixed-ending file bytes matters.

The formatter binary is verified only on native Linux x86_64 GNU. Install or
build it separately using the [repository instructions](../../README.md#build-from-source).
This extension does not download a formatter. It contains no grammar, Rust or
WebAssembly code, and adds no syntax highlighting, language server or completion
provider.

## Install locally

If another extension already provides a language named `Revo`, keep that
extension and merge only this package's [settings.json](settings.json). Check
that it recognizes both suffixes. Do not install this metadata extension beside
another Revo language definition, since duplicate registration can choose the
wrong language configuration and displace existing language tooling.

Otherwise, in Zed's Extensions page choose `Install Dev Extension`, or run
`zed: install dev extension` from the command palette. Select this checkout's
`editors/zed` directory, which contains `extension.toml`. This is a local dev
extension, not a registry release. See Zed's
[dev extension instructions](https://zed.dev/docs/extensions/developing-extensions).

Open user settings with `zed: open settings file`, or edit the intended project's
`.zed/settings.json`. Merge the `languages.Revo` object below into your existing
settings. Keep other languages and unrelated Revo settings. Installing the
extension does not load `settings.json` automatically or edit your settings.

```json
{
  "languages": {
    "Revo": {
      "formatter": {
        "external": {
          "command": "revofmt",
          "arguments": ["--indent-width", "2", "--line-width", "80", "-"]
        }
      },
      "format_on_save": "off",
      "remove_trailing_whitespace_on_save": false,
      "ensure_final_newline_on_save": false
    }
  }
}
```

Open a `.rv` or `.revo` file and confirm the status bar language is `Revo`.
Run `editor: format` from the command palette to format the whole buffer.
Range formatting is outside this integration's scope. For an unnamed buffer,
select `Revo` manually before formatting.

Both whitespace settings must remain `false`. Despite their names, Zed runs
these whitespace passes before the external command during manual formatting
too. They can remove spaces inside opaque multiline literals or change the
buffer even if the CLI rejects the source. The CLI must own whitespace changes.
Also avoid Revo formatter chains, inherited format code actions, or enforced
CRLF settings when relying on the LF preservation scope. Review your effective
settings if you have global formatting customizations.

## Customize the command

Change `languages.Revo.formatter.external.command` to an absolute executable
path if `revofmt` is not on Zed's PATH. Put the executable path alone in
`command`, including any spaces in the path. Keep arguments in the array;
shell expansions such as `~`, pipes and command substitutions are not supported.
Zed runs the executable directly and sends buffer text to stdin. See the
[external formatter setting](https://zed.dev/docs/reference/all-settings#formatter).

Change the string following `--indent-width` to a value from `1` through `8`,
and the string following `--line-width` to a value from `20` through `240`.
For example, four spaces and a 100-column soft target use
`["--indent-width", "4", "--line-width", "100", "-"]`. These CLI settings
are independent of Zed's tab size and wrapping preferences. Keep the final `-`
so formatting reads the unsaved buffer, rather than a file on disk.

To opt into whole-buffer formatting on save, change only
`languages.Revo.format_on_save` to `"on"`. Keep both native whitespace passes
disabled. The shipped snippet explicitly uses `"off"`, including when your
global settings enable save formatting.

## Preservation and host limits

The CLI validates every returned result with the pinned Revo frontend, checking
token/comment bytes, syntax equivalence modulo coordinates and idempotence.
Empty source stays empty. Syntax or validation failures return exit code 2,
stderr diagnostics and no formatted output.

Zed owns process execution and edit application. Its current
[native formatting implementation](https://github.com/zed-industries/zed/blob/main/crates/project/src/lsp_store.rs)
checks the exit status before creating a diff, includes stderr in process
errors, and aborts application if the formatting transaction is no longer at
the top of the undo stack. These are native host behaviors, not custom guards
provided by this package. There is no package-specific cancellation or timeout
control.

Zed sends rope text to stdin. Its
[Buffer::diff implementation](https://github.com/zed-industries/zed/blob/main/crates/language/src/buffer.rs)
detects and normalizes output endings before applying edits and records a
buffer-wide line ending. Raw CRLF or mixed-ending source bytes may already be
changed before the CLI runs. A successful direct CRLF subprocess check cannot
establish preservation through Zed. Keep this limitation in mind before enabling
save formatting.

The current
[LanguageConfig type](https://github.com/zed-industries/zed/blob/main/crates/language_core/src/language_config.rs)
has an optional grammar, so this metadata-only language deliberately omits one.
Zed's language-extension guide still describes a grammar as required; this
package follows the inspected current source. Its
[extension builder](https://github.com/zed-industries/zed/blob/main/crates/extension/src/extension_builder.rs)
discovers `languages/revo/config.toml` and compiles Rust only when a Rust library
is present. This compatibility assessment used current source on 2026-10-06,
not an automated installation in a running Zed editor.

## Verification

From the repository root, with Python 3.11 or newer:

```sh
python3 editors/zed/tests/run.py
REVOFMT_BIN=/absolute/path/to/revofmt python3 editors/zed/tests/run.py
```

The runner defaults to this repository's `target/debug/revofmt` and requires an
existing executable. It uses only Python's standard library. The checks parse
the TOML and JSON, verify recognition and preservation settings, and execute the
configured argument array against the real CLI with raw byte transport.
They cover LF layout, Unicode, opaque literal whitespace, CRLF and mixed-ending
CLI output, empty input, idempotence, syntax and option failures, resource
admission, and stdin formatting without disk writes. Every successful CLI call
runs the formatter's built-in preservation validation.

These are metadata and subprocess tests. They do not test native language
registration, editor buffer preservation, undo, cancellation or stale edit
application. The local `zeditor --version` reported 1.22.0; no native host smoke
test was performed, and no personal editor configuration was changed.

For a manual host check after installing and merging settings, use an LF file
with `let x=1`, make an unsaved edit, run `editor: format`, and confirm only the
buffer changes. Format twice, then undo. Confirm malformed source reports an
error without changing the buffer, and that an empty buffer stays empty. Check
both suffixes and verify save formatting remains off before opting in.
