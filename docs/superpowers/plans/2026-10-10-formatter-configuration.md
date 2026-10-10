# Formatter Configuration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add tab indentation, a configurable blank-line limit and a discovered `revofmt.toml` to revofmt 0.2.0, then make the Neovim, VS Code and Zed adapters honor it.

**Architecture:** The library gains two `FormatOptions` fields consumed by the renderer (tabs) and layout builder (blank lines). A new binary-only `src/config.rs` discovers, reads and parses `revofmt.toml` and resolves per-input options; `src/cli.rs` asks it for each input. Adapters pass `--prefer-config` and the buffer path so a project file overrides their settings.

**Tech Stack:** Rust 2024, `toml` 1.x (binary only), Zig 0.17.0 bridge (unchanged), Lua (Neovim), Node.js (VS Code, Zed server), Rust/WASM (Zed extension).

**Spec:** [docs/superpowers/specs/2026-10-10-formatter-configuration-design.md](../specs/2026-10-10-formatter-configuration-design.md)

## Global Constraints

- Ranges and defaults: `indent_width` 1..=8 (2), `line_width` 20..=240 (80), `indent_style` `space`|`tab` (`space`), `max_blank_lines` 0..=8 (1).
- Default options produce the same bytes as the starting commit `f9c95329`. Exit codes stay 0 success, 1 check differences, 2 usage/I/O/syntax/validation.
- Crate version becomes `0.2.0`.
- Configuration file name `revofmt.toml`; read at most 65,536 bytes plus one; UTF-8.
- `toml` major version 1, a dependency of the binary only. Never enable its `unbounded` feature.
- Every returned result keeps the exact token/comment tape, the AST modulo coordinates and idempotence. New option values go through the real preservation interface in tests.
- Do not modify `vendor/revo/`, corpus input bytes or license notices.
- Formatting regressions go in `src/tests/formatting.rs`; corpus checks in `src/tests/corpus.rs`; CLI process tests in `tests/cli.rs`. Do not import private Rust items into integration tests.
- Build environment: `export ZIG=/home/idan/GitRepo/revo-formatter/.tools/zig-x86_64-linux-0.17.0/zig` before any cargo command. Add crates with `--offline`; builds never download tools.
- Adapter argv order: `--prefer-config [--stdin-filepath P] --indent-width N --line-width N --indent-style S --max-blank-lines N -`.
- Tagging, publishing releases and pushing require explicit user approval at that time.

## Review Focus

- A bare relative filename (`revofmt --check a.rv` run inside the project) must find `./revofmt.toml`; `Path::parent` returns `""` for it. Test: Task 7 `relative_inputs_resolve_against_the_working_directory`.
- An empty `revofmt.toml`, or one saved with CRLF line endings, must parse (empty file means all defaults). Test: Task 6 `empty_and_crlf_files_parse`.
- An out-of-range config value overridden by a flag is not an error in default precedence, but is under `--prefer-config` or without the flag. Test: Task 7 `overridden_invalid_config_values`.
- An input reached through a symlinked directory uses the configuration around its canonical location. Test: Task 7 `symlinked_directories_use_the_canonical_location`.
- A file under a configured project, edited as a new unsaved buffer in a directory that does not exist yet, still finds the project configuration. Test: Task 7 `stdin_filepath_discovers_from_nonexistent_and_relative_paths`.

Test temp directories live under `std::env::temp_dir()`. Tests that assert "no configuration found" first assert that no ancestor of the temp directory contains `revofmt.toml`, so a stray file fails with a clear message instead of a confusing layout diff.

---

## Part A: formatter (this repository)

### Task 1: Public options and version

**Files:**
- Modify: `src/lib.rs:12-37`, `Cargo.toml` (version), `Cargo.lock`
- Modify: struct literals the compiler rejects in `src/tests/*.rs`, `tests/cli.rs:130`
- Modify: `docs/formatter.md` (Rust library usage example)
- Test: `src/tests/formatting.rs`

**Interfaces:**
- Produces: `pub enum IndentStyle { #[default] Space, Tab }` deriving `Debug, Clone, Copy, PartialEq, Eq, Default`; `FormatOptions` fields `indent_style: IndentStyle`, `max_blank_lines: usize`; `FormatOptions::validate` checks all four ranges.

- [ ] **Step 1: Write the failing test** in `src/tests/formatting.rs` (import `IndentStyle` from `crate`)

```rust
#[test]
fn new_options_default_to_current_layout_and_validate_ranges() {
    let defaults = FormatOptions::default();
    assert_eq!(defaults.indent_style, IndentStyle::Space);
    assert_eq!(defaults.max_blank_lines, 1);
    for max_blank_lines in [0, 8] {
        assert!(FormatOptions { max_blank_lines, ..defaults }.validate().is_ok());
    }
    assert!(matches!(
        FormatOptions { max_blank_lines: 9, ..defaults }.validate(),
        Err(FormatError::InvalidOptions(_))
    ));
}
```

- [ ] **Step 2: Run it and confirm it fails to compile**

Run: `cargo test --lib new_options_default`
Expected: error, `IndentStyle` not found.

- [ ] **Step 3: Implement** `IndentStyle`, the two fields, defaults and validation in `src/lib.rs`. The `InvalidOptions` message becomes `"indent width must be 1..=8, line width 20..=240 and max blank lines 0..=8"`. Add `..FormatOptions::default()` (or `..Default::default()`) to every struct literal the compiler rejects. Set `version = "0.2.0"` in `Cargo.toml`. Update the `docs/formatter.md` library example to set all four fields and import `IndentStyle`.

- [ ] **Step 4: Run the library and CLI tests**

Run: `cargo test --all-targets`
Expected: all pass, including `new_options_default_to_current_layout_and_validate_ranges`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src tests docs/formatter.md
git commit -m "Add indent style and blank-line options to FormatOptions"
```

### Task 2: Tab indentation

**Files:**
- Modify: `src/document.rs:102-179` (renderer) and its unit test
- Modify: `src/layout.rs:34` (render call)
- Test: `src/tests/formatting.rs`, `src/document.rs` tests

**Interfaces:**
- Consumes: `IndentStyle`, `FormatOptions::{indent_style, indent_width}` (Task 1).
- Produces: `pub(crate) struct Indentation { pub(crate) style: IndentStyle, pub(crate) columns: usize }` (derive `Clone, Copy`) and `pub(crate) fn render(doc: &Doc<'_>, indentation: Indentation, width: usize, ending: &str) -> String` in `src/document.rs`.

- [ ] **Step 1: Write the failing tests** in `src/tests/formatting.rs`

```rust
fn tabs(indent_width: usize, line_width: usize) -> FormatOptions {
    FormatOptions {
        indent_width,
        line_width,
        indent_style: IndentStyle::Tab,
        ..FormatOptions::default()
    }
}

#[test]
fn tab_indentation_writes_one_tab_per_level() {
    check("fn f() do\nif x do\nfoo()\nend\nend", "fn f() do\n\tif x do\n\t\tfoo()\n\tend\nend\n", tabs(2, 80));
    check(
        "do\nlet total=first_value+second_value+third_value\nend",
        "do\n\tlet total = first_value +\n\t\tsecond_value +\n\t\tthird_value\nend\n",
        tabs(4, 30),
    );
    check("let f = match x\n| 1 => alpha()\n| _ => beta()", "let f = match x\n\t| 1 => alpha()\n\t| _ => beta()\n", tabs(4, 80));
    // Conservative fallback (lexical join) inside a block.
    check("do\nlet x=1 .field\nend", "do\n\tlet x=1 .field\nend\n", tabs(2, 80));
}

#[test]
fn tab_indentation_keeps_opaque_interiors() {
    check(
        "do\nlet s=\"\"\"\n    first\n      second\n    \"\"\"\nconsume(s)\nend",
        "do\n\tlet s = \"\"\"\n    first\n      second\n    \"\"\"\n\tconsume(s)\nend\n",
        tabs(2, 80),
    );
    check("do\n#* doc\n   line *#\nfn f() 1\nend", "do\n\t#* doc\n   line *#\n\tfn f() 1\nend\n", tabs(2, 80));
}

#[test]
fn a_tab_counts_as_indent_width_columns_when_fitting() {
    // 4 + 20 columns fits 24; 4 + 21 does not. A one-column tab would fit both.
    check("do\nconsume(alpha, beta)\nend", "do\n\tconsume(alpha, beta)\nend\n", tabs(4, 24));
    check("do\nconsume(alpha, betas)\nend", "do\n\tconsume(\n\t\talpha,\n\t\tbetas\n\t)\nend\n", tabs(4, 24));
    let source = include_str!("../../tests/fixtures/stress/syntax.rv");
    for (line_width, indent_width) in [(24, 4), (80, 2)] {
        let spaces = format(source, &FormatOptions { line_width, indent_width, ..FormatOptions::default() }).unwrap();
        let tabbed = format(source, &tabs(indent_width, line_width)).unwrap();
        // Identical breaks: only leading whitespace may differ.
        assert!(spaces.lines().map(str::trim_start).eq(tabbed.lines().map(str::trim_start)));
    }
}
```

Add to `src/document.rs` tests, beside the existing test (update its two `render` calls to pass `Indentation { style: IndentStyle::Space, columns: 2 }`):

```rust
#[test]
fn tab_indentation_reserves_its_display_width() {
    let head = Doc::fill(
        Doc::Text("if first_argument +"),
        vec![Doc::concat(vec![Doc::Soft(" "), Doc::Text("22")]).group()],
    );
    let doc = head.followed_by(Doc::Text(" do"));
    let tab = Indentation { style: IndentStyle::Tab, columns: 2 };
    assert_eq!(render(&doc, tab, 24, "\n"), "if first_argument +\n\t22 do");
}
```

- [ ] **Step 2: Run them and confirm failure**

Run: `cargo test --lib tab_`
Expected: compile error, `Indentation` not found.

- [ ] **Step 3: Implement** `Indentation` and the new `render` signature. At a pending indent, set `column = depth * columns` and write `depth` tabs for `Tab` or `column` spaces for `Space`. Use `columns` everywhere the renderer used `indent_width` (group fit and `Fill` continuation at `depth + 1`). `layout::layout` passes `Indentation { style: options.indent_style, columns: options.indent_width }`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: all pass. If an expected literal differs, find the cause before changing it. These literals come from the 0.1.2 space output with spaces replaced by tabs.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "Render indentation with tabs when requested"
```

### Task 3: Blank-line limit

**Files:**
- Modify: `src/layout.rs:9-35` (doc comment, `Builder` construction), `src/layout.rs:75-83` (`breaks`)
- Test: `src/tests/formatting.rs`

**Interfaces:**
- Consumes: `FormatOptions::max_blank_lines` (Task 1).
- Produces: `Builder` field `max_newlines: usize` set to `options.max_blank_lines + 1`; `breaks` uses `.clamp(1, self.max_newlines)`.

- [ ] **Step 1: Write the failing tests**

```rust
fn blank_lines(max_blank_lines: usize) -> FormatOptions {
    FormatOptions { max_blank_lines, ..FormatOptions::default() }
}

#[test]
fn blank_line_limit_caps_statement_and_block_gaps() {
    let statements = "let a=1\n\n\n\nlet b=2";
    check(statements, "let a = 1\nlet b = 2\n", blank_lines(0));
    check(statements, "let a = 1\n\nlet b = 2\n", blank_lines(1));
    check(statements, "let a = 1\n\n\nlet b = 2\n", blank_lines(2));
    let block = "do\n\n\nfoo()\n\n\n\nbar()\n\nend";
    check(block, "do\n  foo()\n  bar()\nend\n", blank_lines(0));
    check(block, "do\n\n  foo()\n\n  bar()\n\nend\n", blank_lines(1));
    check(block, "do\n\n\n  foo()\n\n\n  bar()\n\nend\n", blank_lines(2));
}

#[test]
fn blank_line_limit_applies_to_lists_comments_and_crlf() {
    let list = "consume(first,\n\n\nsecond)";
    // With no blank line left, the fixed-point pass recompacts the list.
    check(list, "consume(first, second)\n", blank_lines(0));
    check(list, "consume(\n  first,\n\n\n  second\n)\n", blank_lines(2));
    let comment = "foo() # note\n\n\n\nbar()";
    check(comment, "foo() # note\nbar()\n", blank_lines(0));
    check(comment, "foo() # note\n\n\nbar()\n", blank_lines(2));
    let crlf = "let a=1\r\n\r\n\r\n\r\nlet b=2";
    check(crlf, "let a = 1\r\nlet b = 2\r\n", blank_lines(0));
    check(crlf, "let a = 1\r\n\r\n\r\nlet b = 2\r\n", blank_lines(2));
}
```

- [ ] **Step 2: Run them and confirm failure**

Run: `cargo test --lib blank_line_limit`
Expected: FAIL; limits 0 and 2 produce the limit-1 output.

- [ ] **Step 3: Implement** the `max_newlines` field and clamp. Change the `layout` doc comment from "capping blank lines at one" to "capping blank lines at the configured limit".

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "Make the blank-line cap configurable"
```

### Task 4: Corpus and stress matrices

**Files:**
- Modify: `src/tests/corpus.rs:92-147` (`combinations`, golden match, counts), `src/tests/corpus.rs:180-268` (`stress_options`, count assertions)
- Modify: `docs/formatter.md:158-180`, `tests/fixtures/stress/README.md:33-36`

**Interfaces:**
- Consumes: `IndentStyle`, both new fields.

- [ ] **Step 1: Change the matrices and assertions**
  - `combinations()` yields widths `[24, 80, 120]` × indent widths `[2, 4]` × `[IndentStyle::Space, IndentStyle::Tab]` × `max_blank_lines` `[0, 1, 2]`. The corpus test asserts `cases == 720` and `goldens == 4`. `expected_output` returns a golden only for `IndentStyle::Space` with `max_blank_lines == 1`.
  - `stress_options()` yields widths `[20, 24, 40, 80, 120, 240]` × `[1, 2, 4, 8]` × both styles × `[0, 1, 2]` (144 sets).
  - Assert `check_stress_source` counts: file-read match `288`, complete file `288`, sections `14_688` across `51` sections, composed `62_208`. Together that is 77,472.

- [ ] **Step 2: Run debug and release corpus tests**

Run: `cargo test --lib tests::corpus && cargo test --release --lib tests::corpus`
Expected: 8 passed in each. Record both durations for Task 8; the 2026-10-10 baseline was 2.64 s debug and 0.77 s release.

- [ ] **Step 3: Update counts in docs.** In `docs/formatter.md`, describe the corpus dimensions and change 120 to 720. Change the stress numbers from 12,864 to 77,184, from 48 to 288 and from 12,912 to 77,472, and mention the indent-style and blank-line dimensions. Apply the same stress number changes in `tests/fixtures/stress/README.md`. Do not edit dated verification records.

- [ ] **Step 4: Commit**

```bash
git add src/tests/corpus.rs docs/formatter.md tests/fixtures/stress/README.md
git commit -m "Cover indent styles and blank-line limits in corpus checks"
```

### Task 5: CLI layout flags

**Files:**
- Modify: `src/cli.rs:11-29` (HELP), `src/cli.rs:45-100` (`parse`)
- Modify: `README.md` and `docs/formatter.md` (CLI usage and defaults paragraph)
- Test: `tests/cli.rs`

**Interfaces:**
- Consumes: `IndentStyle`, both new fields.
- Produces: flags `--indent-style space|tab` and `--max-blank-lines N` that set `Arguments.options`. Task 7 replaces this storage.

- [ ] **Step 1: Write the failing tests** in `tests/cli.rs`

```rust
#[test]
fn indent_style_and_blank_line_flags() {
    let output = run(&["--indent-style", "tab"], "do\nfoo()\nend");
    status(&output, 0);
    assert_eq!(output.stdout, b"do\n\tfoo()\nend\n");
    let output = run(&["--max-blank-lines", "0"], "let a=1\n\n\nlet b=2");
    status(&output, 0);
    assert_eq!(output.stdout, b"let a = 1\nlet b = 2\n");
    let output = run(&["--max-blank-lines", "2", "--indent-style", "space"], "let a=1\n\n\n\nlet b=2");
    status(&output, 0);
    assert_eq!(output.stdout, b"let a = 1\n\n\nlet b = 2\n");
}
```

Add these to the argument lists in `rejects_invalid_arguments_without_source`: `["--indent-style"]`, `["--indent-style", "tabs"]`, `["--max-blank-lines"]`, `["--max-blank-lines", "9"]`, `["--max-blank-lines", "-1"]`. Add `"--indent-style"` and `"--max-blank-lines"` to the option names checked in `help_version_and_option_overrides`.

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test --test cli`
Expected: FAIL with `unrecognized option: --indent-style`.

- [ ] **Step 3: Implement** parsing.
  - An `--indent-style` value other than `space` or `tab` is the error `"--indent-style requires space or tab"`.
  - A missing or non-integer `--max-blank-lines` value is the error `"--max-blank-lines requires a non-negative integer"`.
  - After each layout flag, run `options.validate()`, as the existing width flags do.
  - Add these HELP lines:

```
  --indent-width N    Columns per indentation level (1 through 8; default 2)
  --indent-style S    Indent with space or tab (default space)
  --line-width N      Target display columns (20 through 240; default 80)
  --max-blank-lines N Consecutive blank lines kept (0 through 8; default 1)
```

  Then update the README options and the `docs/formatter.md` defaults paragraph: tabs write one tab per level and count `--indent-width` columns; blank lines accept 0 through 8 with default 1.

- [ ] **Step 4: Run the tests**

Run: `cargo test --all-targets`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs tests/cli.rs README.md docs/formatter.md
git commit -m "Add --indent-style and --max-blank-lines flags"
```

### Task 6: Configuration module

**Files:**
- Create: `src/config.rs` (code and `#[cfg(test)] mod tests`)
- Modify: `src/main.rs` (`mod config;`), `Cargo.toml`, `Cargo.lock`

**Interfaces:**
- Consumes: `revofmt::{FormatOptions, IndentStyle}`.
- Produces, in `src/config.rs`:
  - `pub(crate) const FILE_NAME: &str = "revofmt.toml";`
  - `#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)] pub(crate) struct Layout { pub(crate) indent_width: Option<usize>, pub(crate) line_width: Option<usize>, pub(crate) indent_style: Option<IndentStyle>, pub(crate) max_blank_lines: Option<usize> }`
  - `impl Layout { pub(crate) fn over(self, base: FormatOptions) -> FormatOptions }`, where each `Some` replaces the base value.
  - `pub(crate) fn parse(text: &str) -> Result<Layout, String>`; messages omit the path.
  - `pub(crate) struct Resolver` with `pub(crate) fn new(flags: Layout, prefer_config: bool, discover: bool) -> Self` and `pub(crate) fn options(&mut self, anchor: Option<&Path>) -> Result<FormatOptions, String>`. `anchor` is the input path or `--stdin-filepath`; `None` skips discovery. Errors are `"{config path}: {message}"`.

Resolution in `options`, per the spec's precedence table, with `d = FormatOptions::default()`:

| Discovery result | `prefer_config` false | `prefer_config` true |
| --- | --- | --- |
| skipped (`!discover` or `anchor` is `None`) or nothing found | `flags.over(d)` | `flags.over(d)` |
| found `file` | `flags.over(file.over(d))` | `file.over(d)` |

Validate the result. On failure with a file found, return `"{path}: {validation message}"`.

Discovery: start from `anchor.parent()`, treating an empty parent as `.`. Join it to `std::env::current_dir()` and canonicalize, falling back to the joined lexical path on error. Walk `ancestors()`.
- If `symlink_metadata(dir/FILE_NAME)` reports `NotFound`, continue upward.
- Any other entry must be readable through `fs::metadata` as a regular file, or it is an error.
- Read at most 65,536 bytes plus one; a larger file is an error, and the contents must be UTF-8.
- Cache each visited directory's result (`Result<Option<(PathBuf, Layout)>, String>`) in a `HashMap<PathBuf, _>`.

- [ ] **Step 1: Add the dependency**

Run: `cargo add --offline toml@1 --no-default-features --features parse,serde,std`
Expected: `Cargo.toml` gains `toml` without `unbounded`, and `Cargo.lock` resolves from the local cache.

- [ ] **Step 2: Write the failing unit tests** in `src/config.rs`. Use a local temp-directory helper modeled on `tests/cli.rs::TempDir`.

```rust
#[test]
fn parses_every_key() {
    let text = "indent_width = 4\nline_width = 100\nindent_style = \"tab\"\nmax_blank_lines = 2\n";
    assert_eq!(parse(text).unwrap(), Layout {
        indent_width: Some(4), line_width: Some(100),
        indent_style: Some(IndentStyle::Tab), max_blank_lines: Some(2),
    });
}

#[test]
fn empty_and_crlf_files_parse() {
    assert_eq!(parse("").unwrap(), Layout::default());
    assert_eq!(parse("# comment\r\nline_width = 90\r\n").unwrap().line_width, Some(90));
}

#[test]
fn rejects_unknown_keys_wrong_types_and_styles() {
    for text in ["indnet_width = 4", "indent_width = \"4\"", "indent_width = 4.0",
                 "indent_width = -1", "indent_style = \"tabs\"", "[layout]\nindent_width = 4"] {
        assert!(parse(text).is_err(), "{text}");
    }
}

#[test]
fn deep_nesting_is_an_error_on_a_two_mib_thread() {
    let text = format!("indent_width = {}", "[".repeat(60_000));
    let result = std::thread::Builder::new().stack_size(2 * 1024 * 1024)
        .spawn(move || parse(&text)).unwrap().join().unwrap();
    assert!(result.is_err());
}
```

Resolver tests, each using real files:
  - `nearest_configuration_wins`: `root/revofmt.toml` sets `line_width = 100` and `root/sub/revofmt.toml` sets `line_width = 60`. An anchor at `root/sub/deeper/a.rv` (the `deeper` directory exists) gives 60, and `root/b.rv` gives 100.
  - `precedence_follows_the_spec_table`: the file sets `indent_style = "tab"` and `line_width = 24`; the flags set `line_width: Some(80)` and `max_blank_lines: Some(0)`.
    - Default precedence gives tab, 80, 0.
    - `prefer_config` gives tab, 24, 1.
    - `discover = false` gives space, 80, 0.
    - An anchor of `None` gives space, 80, 0.
  - `no_configuration_uses_flags_and_defaults`: assert the temp-directory precondition from Review Focus, then the result is `flags.over(default)`.
  - `out_of_range_values_name_the_configuration`: a file setting `max_blank_lines = 9` produces an error containing the config path and `"max blank lines"`.
  - `unreadable_entries_are_errors`: a 65,537-byte file, invalid UTF-8 (`[0xff]`), a directory named `revofmt.toml` and a dangling symlink each produce an `Err` containing the path. A symlink to a valid file is followed.
  - `results_are_cached_per_directory`: resolve `dir/a.rv`, delete `dir/revofmt.toml`, then resolve `dir/b.rv` with the same `Resolver`. Both get the file's values.

- [ ] **Step 3: Run and confirm failure**

Run: `cargo test --bin revofmt config::`
Expected: compile errors for the missing items.

- [ ] **Step 4: Implement** `src/config.rs`. Deserialize into a private struct with `#[serde(deny_unknown_fields)]`, `Option<usize>` integers and `indent_style: Option<String>`, then map `"space"` and `"tab"` to `IndentStyle`. Declare `mod config;` in `src/main.rs`. Until Task 7 uses the module, add `#[allow(dead_code)]` on the module declaration with a comment pointing to Task 7, and remove it there.

- [ ] **Step 5: Run the tests**

Run: `cargo test --bin revofmt config:: && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/config.rs src/main.rs
git commit -m "Add revofmt.toml discovery and parsing"
```

### Task 7: CLI configuration wiring

**Files:**
- Modify: `src/cli.rs` (HELP, `Arguments`, `parse`, `run`, `read_and_format`, `write_files`), `src/main.rs` (remove the `allow`)
- Modify: `README.md`, `docs/formatter.md`, `docs/architecture.md`, `CONTEXT.md`
- Test: `tests/cli.rs`

**Interfaces:**
- Consumes: `config::{Layout, Resolver}` (Task 6).
- Produces: `Arguments { mode, flags: Layout, prefer_config: bool, no_config: bool, stdin_filepath: Option<PathBuf>, inputs }`; `fn read_and_format(path: &Path, resolver: &mut Resolver, stdin_filepath: Option<&Path>) -> Result<(String, String), String>`. The anchor is `stdin_filepath` for `-` and `Some(path)` otherwise.

- [ ] **Step 1: Write the failing process tests** in `tests/cli.rs`. Add `fn run_in(dir: &Path, args: &[&str], stdin: &str) -> Output`, which sets `Command::current_dir`, alongside `invoke`.
  - `configuration_precedence_rows`: `dir/revofmt.toml` contains `indent_style = "tab"\nline_width = 24\n`, and `dir/a.rv` contains `do\nconsume(first_argument, second)\nend`. Running on `a.rv`:

    | Flags | Expected stdout |
    | --- | --- |
    | none | `do\n\tconsume(\n\t\tfirst_argument,\n\t\tsecond\n\t)\nend\n` |
    | `--line-width 80` | `do\n\tconsume(first_argument, second)\nend\n` |
    | `--prefer-config --line-width 80 --indent-style space` | same as no flags |
    | `--no-config` | `do\n  consume(first_argument, second)\nend\n` |
    | `--no-config --prefer-config --indent-style tab --line-width 80` | `do\n\tconsume(first_argument, second)\nend\n` |

    `dir/b.rv` contains `let a=1\n\n\nlet b=2`. `--prefer-config --max-blank-lines 0` gives `let a = 1\n\nlet b = 2\n` because the default fills the missing key. `--max-blank-lines 0` gives `let a = 1\nlet b = 2\n`.
  - `stdin_filepath_discovers_from_nonexistent_and_relative_paths`: with the same `revofmt.toml`, stdin `do\nfoo()\nend` gives `do\n\tfoo()\nend\n` in each of these cases:
    - `--stdin-filepath <dir>/missing/new.rv`, where `missing` doesn't exist
    - `run_in(dir, ["--stdin-filepath", "sub/x.rv"])`
    - `--check --stdin-filepath <dir>/x.rv`, with already-tabbed stdin, exits 0

    `run_in(dir, [])` without the flag gives `do\n  foo()\nend\n`.
  - `stdin_filepath_requires_stdin`: `["--stdin-filepath", "x.rv", "a.rv"]` and `["--stdin-filepath"]` each exit 2 with empty stdout.
  - `batches_resolve_each_input_separately`: `one/revofmt.toml` sets `indent_style = "tab"` and `two/revofmt.toml` sets `max_blank_lines = 0`. Write `one/a.rv` (`do\nfoo()\nend`) and `two/b.rv` (`let a=1\n\n\nlet b=2`) with `--write`, which exits 0. The files then contain `do\n\tfoo()\nend\n` and `let a = 1\nlet b = 2\n`.
  - `malformed_configuration_prevents_every_write`: `ok/a.rv` is unformatted, and `bad/revofmt.toml` contains `indnet_width = 4` next to `bad/b.rv`. `--write ok/a.rv bad/b.rv` exits 2, `ok/a.rv` is unchanged, and stderr contains the `bad/revofmt.toml` path.
  - `check_continues_past_configuration_errors`: `--check bad/b.rv ok/a.rv` exits 2, and stderr contains both the config path and `requires formatting` for `ok/a.rv`.
  - `relative_inputs_resolve_against_the_working_directory`: `dir/revofmt.toml` sets `indent_style = "tab"`, and `dir/a.rv` is `do\n\tfoo()\nend\n`. `run_in(dir, ["--check", "a.rv"])` exits 0.
  - `overridden_invalid_config_values`: `revofmt.toml` sets `indent_width = 20`, and the input is `dir/a.rv`.
    - `--indent-width 2 a.rv` exits 0.
    - Without the flag it exits 2, and stderr contains the config path.
    - `--prefer-config --indent-width 2` exits 2.
  - `symlinked_directories_use_the_canonical_location`: `project/revofmt.toml` sets `indent_style = "tab"`, `project/link` is a symlink to `outside/`, and `outside/a.rv` contains `do\nfoo()\nend`. Printing `project/link/a.rv` gives `do\n  foo()\nend\n`.
  - Invalid flags are still rejected under `--prefer-config`: add `["--prefer-config", "--indent-width", "0"]` to `rejects_invalid_arguments_without_source`.
  - Help test: check that `--stdin-filepath`, `--prefer-config` and `--no-config` appear.

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test --test cli`
Expected: FAIL with `unrecognized option: --stdin-filepath` and config-less output.

- [ ] **Step 3: Implement the CLI wiring.**
  - Parse the new flags. `--stdin-filepath` takes an `OsString` value, and a missing value is the error `"--stdin-filepath requires a path"`.
  - Validate layout flags through `flags.over(FormatOptions::default()).validate()`.
  - After inputs default to `-`, reject a `--stdin-filepath` that comes with any non-`-` input, with the error `"--stdin-filepath requires stdin input"`.
  - Build one `Resolver::new(flags, prefer_config, !no_config)` per run and pass it to every `read_and_format`.
  - Resolve options before formatting each input. For `--write`, this keeps config errors inside batch prevalidation.
  - Diagnostics keep `input_name(path)` as the prefix.
  - Replace HELP with:

```
revofmt [OPTIONS] [FILE]
revofmt --check [OPTIONS] [FILE...]
revofmt --write [OPTIONS] FILE...

Read stdin when FILE is absent or is -. Print formatted source by default.

Options:
  --check             Report changed inputs without editing
  --write             Atomically replace changed files, preserving permissions
  --indent-width N    Columns per indentation level (1 through 8; default 2)
  --indent-style S    Indent with space or tab (default space)
  --line-width N      Target display columns (20 through 240; default 80)
  --max-blank-lines N Consecutive blank lines kept (0 through 8; default 1)
  --stdin-filepath P  Find revofmt.toml from P when reading stdin
  --prefer-config     Ignore layout flags when revofmt.toml applies
  --no-config         Do not search for revofmt.toml
  --help              Print this help
  --version           Print formatter version and pinned syntax revision
  --                  Treat following arguments as file paths

Layout flags override revofmt.toml, found in each input's directory or its
nearest parent. A tab counts as --indent-width columns.
Print mode accepts one input. --check and --write are mutually exclusive.
Write mode rejects stdin and symlinks. Exit codes: 0 success, 1 check
differences, 2 usage/I/O/syntax/validation error. Diagnostics go to stderr.
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --all-targets && cargo clippy --all-targets -- -D warnings`
Expected: all pass.

- [ ] **Step 5: Update the docs**
  - `docs/formatter.md`: add a "Configuration" section after CLI usage with the file example, discovery, precedence table, `--stdin-filepath` rules and error behavior, all from the spec. Delete "There is no directory discovery or configuration file support in this version."
  - `README.md`: a short configuration example linking to that section.
  - `docs/architecture.md`: add an ownership row, `src/config.rs`: "Configuration discovery, bounded reading, TOML parsing and per-input option resolution". The `src/cli.rs` row gains "per-input options from configuration".
  - `CONTEXT.md`: under Language, add **Project configuration**: "A `revofmt.toml` file found in an input's directory or its nearest ancestor. It supplies layout options for inputs beneath it. Command-line flags override it unless `--prefer-config` is given."

- [ ] **Step 6: Commit**

```bash
git add src tests/cli.rs README.md docs/formatter.md docs/architecture.md CONTEXT.md
git commit -m "Resolve layout options from revofmt.toml per input"
```

### Task 8: Formatter verification and release gate

**Files:**
- Create: `docs/verification/2026-10-<DD>-formatter-configuration.md` (date of the run)
- Modify: `docs/README.md` (plans table row and current-guides link)

- [ ] **Step 1: Run the full verification** from the README "development and verification" section, plus `cargo package --offline --allow-dirty`:

```sh
cargo test --all-targets
cargo test --release --all-targets
cargo test --doc
cargo clippy --all-targets -- -D warnings
cargo fmt --check
"$ZIG" fmt --check bridge.zig bridge/*.zig
"$ZIG" test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter indexed --test-filter 'input limits:' --cache-dir target/zig-test-cache
cargo build --release
(cd vendor/revo && sha256sum --check SHA256SUMS)
(cd tests/fixtures/upstream && sha256sum --check SHA256SUMS)
cargo package --offline --allow-dirty
```

Expected: every command exits 0.

- [ ] **Step 2: Default-output regression check.** Build the starting commit with `git worktree add /tmp/revofmt-base f9c95329` followed by `cargo build --release` there (with `ZIG` set). Format every `tests/fixtures/upstream/*.rv` and `tests/fixtures/stress/*.rv` with that binary and with the new release binary at default options. Every file must produce identical bytes. Then run `git worktree remove /tmp/revofmt-base`.

- [ ] **Step 3: Write the verification record.** Include the commands, test counts, corpus timings from Task 4, the default-output comparison and the limits: no adapter changes yet and no native editor session. Add the spec and plan row to the `docs/README.md` plans table and the record to current guides.

- [ ] **Step 4: Commit**

```bash
git add docs
git commit -m "Record formatter configuration verification"
```

- [ ] **Step 5: STOP. Ask the user to approve tagging and publishing v0.2.0** (binary asset `revofmt-linux-x86_64-gnu` and `SHA256SUMS`, as for v0.1.2). Part B needs the published checksum. Do not tag, push or publish without that approval.

## Part B: adapters (after v0.2.0 is published)

Work in each repository on a new branch `formatter-configuration`. Run `scripts/verify` with `REVOFMT_BIN` pointing to the published v0.2.0 binary. Releasing an adapter follows its `docs/publishing.md` and needs the same explicit approval as Task 8.

### Task 9: Formatter install links

**Files:**
- Modify: `README.md` install steps (`v0.1.2` URLs and checksum)

- [ ] **Step 1:** Replace the v0.1.2 download and `SHA256SUMS` links with v0.2.0, and confirm the documented `sha256sum --check` passes against the published assets.
- [ ] **Step 2: Commit** `git commit -am "Point installation at revofmt 0.2.0"`

### Task 10: Neovim adapter (`~/GitRepo/revofmt.nvim`)

**Files:**
- Modify: `lua/revofmt/init.lua` (defaults, `setup` validation, `M.format` path), `lua/revofmt/transport.lua:59-62` (argv), `lua/revofmt/release.lua`
- Modify: `tests/fixture.py:9`, `tests/run.lua`, `README.md`, `doc/revofmt.txt`

**Interfaces:**
- Produces: `transport.arguments(config) -> string[]` in `lua/revofmt/transport.lua`. `config.path` is an absolute path or `nil`. `M.format` passes `path` in `process_config`.

- [ ] **Step 1: Write the failing tests** in `tests/run.lua`:
  - `passes indent style and blank-line settings to the real CLI`: `plugin({ indent_style = 'tab', max_blank_lines = 0 })` on the buffer `{ 'do', 'foo()', '', '', 'bar()', 'end' }` gives the bytes `'do\n\tfoo()\n\tbar()\nend\n'`.
  - `a project revofmt.toml overrides adapter layout settings`: create `dir = vim.fn.tempname()` containing `revofmt.toml` with `indent_style = "tab"`. Name a buffer `{ 'do', 'foo()', 'end' }` as `dir .. '/a.rv'` without writing it, then use `plugin({ indent_style = 'space' })`. The result is `'do\n\tfoo()\nend\n'`.
  - Add `{ indent_style = 'tabs' }`, `{ max_blank_lines = -1 }`, `{ max_blank_lines = 9 }` and `{ max_blank_lines = 1.5 }` to the validation test.
  - `tests/fixture.py` accepts `["--prefer-config", "--indent-width", "2", "--line-width", "80", "--indent-style", "space", "--max-blank-lines", "1", "-"]`, optionally with `"--stdin-filepath", P` inserted after `--prefer-config`.
- [ ] **Step 2: Run** `REVOFMT_BIN=… scripts/verify` and confirm the new tests fail.
- [ ] **Step 3: Implement.**
  - Add defaults `indent_style = 'space'` and `max_blank_lines = 1`.
  - Validation errors are `'revofmt: invalid indent_style'` and `'revofmt: invalid max_blank_lines'` (an integer from 0 through 8).
  - Send a path only when `vim.bo[buf].buftype == ''` and `nvim_buf_get_name(buf) ~= ''`, using `vim.fn.fnamemodify(name, ':p')`.
  - `transport.arguments` builds the global argv order.
- [ ] **Step 4: Run** `scripts/verify`. Expected: all tests pass.
- [ ] **Step 5: Pin and document.**
  - Set `release.lua` to `version = '0.2.0'` with the v0.2.0 URL, the published `sha256` and the release `source_revision`.
  - README and `doc/revofmt.txt` gain the two settings, the 0.2.0 minimum and "a project `revofmt.toml` overrides these layout settings for files beneath it".
  - Run `scripts/verify` again, including `tests/install.lua`.
- [ ] **Step 6: Commit** `git commit -am "Support revofmt.toml, tab indentation and blank-line limits"`

### Task 11: VS Code adapter (`~/GitRepo/revofmt-vscode`)

**Files:**
- Modify: `package.json` (settings, version per `docs/publishing.md`), `src/settings.js`, `src/provider.js:24-26`, `src/transport.js:17-20`
- Modify: `tests/helpers.js` (defaults gain `indentStyle: 'space'`, `maxBlankLines: 1`), `tests/transport.test.js:26`, `tests/provider.test.js:45`, `.github/workflows/test.yml:25-27`, `README.md`

**Interfaces:**
- Produces: `argumentsFor(settings) -> string[]`, exported from `src/transport.js`. `settings.filePath` is an absolute path or `undefined`.

- [ ] **Step 1: Write the failing tests.**
  - `transport.test.js` args fixture: `controlled()` gives `["--prefer-config","--indent-width","4","--line-width","24","--indent-style","space","--max-blank-lines","1","-"]` with `indentWidth: 4, lineWidth: 24`. With `filePath: '/project/a.rv'` it gives `["--prefer-config","--stdin-filepath","/project/a.rv",…]`.
  - Real-CLI test: create a temp directory with `revofmt.toml` containing `indent_style = "tab"`. `format('do\nfoo()\nend', { ...defaults, filePath: join(dir, 'a.rv') })` returns `'do\n\tfoo()\nend\n'`.
  - `provider.test.js`: a document with `uri: { scheme: 'file', fsPath: '/project/a.rv' }` passes `filePath: '/project/a.rv'` to `run`, and an `untitled` document passes `undefined`. Add `['indentStyle', 'tabs']`, `['maxBlankLines', -1]`, `['maxBlankLines', 9]` and `['maxBlankLines', 1.5]` to the validation list.
- [ ] **Step 2: Run** `REVOFMT_BIN=… scripts/verify` and confirm the failures.
- [ ] **Step 3: Implement.**
  - `package.json`:
    - `revofmt.indentStyle`: string enum `["space","tab"]`, default `space`, `resource` scope, description "Indent with spaces or one tab per level. A project revofmt.toml takes precedence."
    - `revofmt.maxBlankLines`: integer, default 1, minimum 0, maximum 8, `resource` scope.
  - `settings.js` errors:
    - `revofmt.indentStyle must be "space" or "tab"`
    - `revofmt.maxBlankLines must be an integer from 0 to 8`
  - `provider.js` passes `filePath` when `document.uri.scheme === 'file'`.
- [ ] **Step 4: Run** `scripts/verify` and `npm run test:host`. Expected: all pass.
- [ ] **Step 5: Pin and document.** Move the CI pin to v0.2.0 with its published sha256. The README gains the settings, the 0.2.0 minimum and the precedence sentence. Bump the extension version per `docs/publishing.md`.
- [ ] **Step 6: Commit** `git commit -am "Support revofmt.toml, tab indentation and blank-line limits"`

### Task 12: Zed adapter (`~/GitRepo/revofmt-zed`)

**Files:**
- Modify: `src/options.rs` (validation and tests), `server/main.cjs` (options, initialize validation, formatting request), `server/formatter.cjs` (argv, stderr capture), `settings.json`, `tests/lifecycle.test.cjs`, `tests/lsp/fixture.cjs`, `tests/run.py:86-87`, `.github/workflows/test.yml:31-33`, `README.md`
- Release files per `docs/publishing.md`: server version, `src/server_checksums.rs`, `src/lib.rs:9-11`

**Interfaces:**
- Produces:
  - `argumentsFor(options) -> string[]`, exported from `server/formatter.cjs`.
  - `format(source, options, done)` now calls `done(text)` on success and `done(null, diagnostic)` on failure. `diagnostic` is the trimmed stderr text when the CLI exits with code 2 and that text is nonempty, and `undefined` otherwise.

- [ ] **Step 1: Write the failing tests.**
  - Lifecycle args test with `{ indentWidth: 4, lineWidth: 120 }`: the output is `["--prefer-config","--indent-width","4","--line-width","120","--indent-style","space","--max-blank-lines","1","-"]` for an `untitled:` URI. A `file:///project/a.rv` URI inserts `"--stdin-filepath","/project/a.rv"` after `--prefer-config`.
  - New fixture mode `config-error`, which writes `bad revofmt.toml` to stderr and exits 2. The client receives a `window/showMessage` notification with `type: 1` whose message contains `bad revofmt.toml`, and the formatting result is `[]`. If the test client doesn't record server notifications, add a `notifications` array to it.
  - Mode `nonzero` (exit code 2, empty stderr) sends no notification. The existing `nonzero-inherited` test must still settle in under 700 ms while a descendant holds the pipes.
  - `options.rs` tests: `{"maxBlankLines": 9}`, `{"maxBlankLines": -1}` and `{"indentStyle": "tabs"}` are rejected; `{"indentStyle": "tab", "maxBlankLines": 0}` is accepted.
  - `tests/run.py`: the command gains the new arguments. Add a real-CLI test where a temp `revofmt.toml` with `indent_style = "tab"` and a `file:` URI produce tab output.
- [ ] **Step 2: Run** `REVOFMT_BIN=… scripts/verify` and confirm the failures.
- [ ] **Step 3: Implement.**
  - Server defaults gain `indentStyle: 'space'` and `maxBlankLines: 1`. `initialize` validates them, with `Invalid indentStyle` and `Invalid maxBlankLines` for bad values.
  - Map a `file:` URI to `fileURLToPath`. On a conversion error, or for any other scheme, omit the path.
  - In `formatter.cjs`, accumulate decoded stderr. On exit code 2, finish with `done(null, diagnostic)` when the stderr stream ends or 100 ms after exit, whichever comes first, using the stderr collected so far. Do not wait for `close`: inherited pipes can hold it open. Every other failure path is unchanged.
  - In `main.cjs`, when `diagnostic` is defined, send `{ jsonrpc: '2.0', method: 'window/showMessage', params: { type: 1, message: 'revofmt: ' + diagnostic } }` before responding `[]`.
  - `settings.json` gains `"indentStyle": "space"` and `"maxBlankLines": 1`.
- [ ] **Step 4: Run** `scripts/verify`. Expected: all checks pass.
- [ ] **Step 5: Pin, document and prepare the server release.** Move the CI CLI pin to v0.2.0 with its sha256. The README gains the settings, the 0.2.0 minimum and the precedence sentence. Follow `docs/publishing.md` for the next server version: `scripts/package-server --update-checksums`, then the `SERVER_DIRECTORY` and `SERVER_URL` pins and the extension version. Publishing the server archive needs approval.
- [ ] **Step 6: Commit** `git commit -am "Support revofmt.toml, tab indentation and blank-line limits"`

### Task 13: Integration guide and record (this repository)

**Files:**
- Modify: `docs/editors.md` (CLI contract), `docs/README.md`
- Modify: the Task 8 verification record (adapter section)

- [ ] **Step 1: Update `docs/editors.md`.** The contract block becomes the global argv order. Explain `--prefer-config`, that `--stdin-filepath` is sent only for real files, the 0.2.0 minimum, and the new indentation and blank-line ranges.
- [ ] **Step 2: Record the adapter verification.** Add each adapter's `scripts/verify` result against v0.2.0 (and `npm run test:host` for VS Code) to the record, noting that no native Neovim, VS Code or Zed session was exercised beyond those suites.
- [ ] **Step 3: Commit** `git commit -am "Document the configuration-aware editor contract"`
