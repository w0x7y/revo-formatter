# Revo formatter implementation plan

Historical approved design/implementation plan; implementation and its review gates are complete. Original interface sketches and stage-specific instructions below are retained as design history. Use the [current architecture](../../architecture.md) for delivered module boundaries and the [final source check](../../verification/2026-10-06-final-check.md) for verification.

> **Historical execution process:** Implementation used task-by-task development with independent spec and quality reviews. This is a completed plan, not a new delegation instruction.

**Goal:** Build a usable Rust Revo formatter library and CLI with indentation, line-width reflow and compiler-backed preservation checks.

**Architecture:** Rust owns source layout and user interfaces. A pinned upstream Zig frontend is statically linked through a small C ABI and provides lossless token analysis, parse diagnostics and complete structural-equivalence checks. Candidate formatting must retain exact token/comment/literal bytes and parse to the same tree.

**Tech stack:** Rust 2024, Cargo, Zig 0.17.0, upstream Revo frontend, serde and serde_json for a small owned bridge payload. No tree-sitter or external Revo runtime.

**Spec:** `docs/superpowers/specs/2026-10-05-revo-formatter-design.md`

## Global constraints

- Build here in Rust.
- Ship a CLI and reusable Rust library first; editor integration comes later.
- Include line-width reflow in the first version.
- Print formatted code by default; provide `--write` and `--check`.
- Support the syntax accepted by upstream revision `b571298b6fc95bc863548f118354c8d077792f6f`.
- Preserve all literal and comment bytes, including documentation comments, quasiquotes, interpolation and multiline-string internal indentation.
- Non-whitespace token spellings must stay unchanged.
- Never return a candidate that fails validation.
- The guarantee is syntax equivalence modulo source coordinates. Formatting procedural macros is allowed; source-position changes are normal formatting effects and must be documented.
- Formatting the output again must produce identical bytes.
- Defaults are two spaces and 80 columns. Indent width accepts 1 through 8; line width accepts 20 through 240.
- Width is a target, not a guarantee. Long literal/comment contents and syntax that cannot safely break may exceed it.
- Build scripts must not download tools or source implicitly.
- Linux on the current machine is the initial verified platform. Cross-platform release claims require validation on those platforms.
- No publication, remote repository creation or editor extension is part of this work.

## File map

- `Cargo.toml`, `Cargo.lock`, `build.rs`: reproducible Rust package and static frontend build.
- `vendor/revo/`: immutable minimal upstream source closure, revision and MIT notice.
- `bridge/frontend.zig`: C ABI, source-token analysis, diagnostics, AST comparison.
- `src/error.rs`: public formatting error and display implementation.
- `src/oracle.rs`: safe owned Rust wrapper around the bridge; token analysis and comparison.
- `src/lib.rs`: public options, formatting API and upstream revision.
- `src/document.rs`: width-aware document rendering.
- `src/layout.rs`: source-token grouping, spacing, block/list/arm layout and conservative mode.
- `src/main.rs`, `src/cli.rs`: arguments, stdin/stdout and atomic file modes.
- `tests/formatting.rs`, `tests/cli.rs`, `tests/corpus.rs`: focused behavior, process-level behavior and pinned-corpus checks.
- `tests/fixtures/`: small expected-output examples and attributed upstream corpus.
- `README.md`, `LICENSE`, `THIRD_PARTY.md`: usage, constraints, build requirements and provenance.

The upstream vendor files may be large; handwritten modules stay focused. Do not port the whole parser or add dependencies on temporary paths. A local Zig toolchain can be provisioned explicitly under ignored `.tools/`; consumers supply Zig via PATH or the `ZIG` environment variable.

### Task 1: Build the pinned frontend bridge and Rust oracle

**Files:** Create `Cargo.toml`, `Cargo.lock`, `build.rs`, `bridge/frontend.zig`, `src/error.rs`, `src/oracle.rs`, `src/lib.rs`, `vendor/revo/REVISION`, `vendor/revo/LICENSE.txt`, required upstream source files, `THIRD_PARTY.md`, `LICENSE`. Modify `.gitignore` for `/target/` and `/.tools/`. Tests live in `src/oracle.rs` and bridge Zig tests.

**Interfaces:** Produces the following crate-private Rust API and public error type. All analysis ranges are original UTF-8 byte ranges; source tokens include the complete delimiter envelope for every comment/string.

```rust
pub const UPSTREAM_REVISION: &str = "b571298b6fc95bc863548f118354c8d077792f6f";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    InvalidOptions(String),
    Syntax { message: String, offset: usize },
    Validation(String),
}

pub(crate) struct SourceToken {
    pub kind: String,
    pub start: usize,
    pub end: usize,
}

pub(crate) struct SyntaxRegion {
    pub kind: String,
    pub start: usize,
    pub end: usize,
}

pub(crate) struct Analysis {
    pub tokens: Vec<SourceToken>,
    pub regions: Vec<SyntaxRegion>,
}

pub(crate) fn analyze(source: &str) -> Result<Analysis, FormatError>;
pub(crate) fn equivalent(original: &str, candidate: &str) -> Result<bool, FormatError>;
```

`FormatError` implements `Display` and `std::error::Error`. `regions` carries source-backed block/statement/match-arm boundaries useful to layout, excluding synthetic generated nodes and out-of-range spans. Region kinds are `statement`, `block` and `match_arm`. Document their interpretation in `oracle.rs`; source tokens remain the printing authority when AST spans omit surface punctuation.

Stable Zig 0.17.0 is the exact compiler/stdlib pin. The verified Linux x86_64 archive SHA-256 is `1cbe9df9f27e6b78d14ccbca43b6703a404ef79ef1c463de901d7f088d4e2026`. A local toolchain is provisioned under `.tools/zig-x86_64-linux-0.17.0/zig`; build scripts discover it only through an explicit `ZIG` value, not by special-casing this path.

- [x] Write failing oracle tests before implementing the bridge. Establish these behavioral assertions, with actual syntax verified against the pin:

```rust
assert!(analyze("let x = 1 + 2 * 3").is_ok());
assert!(analyze("let x =").is_err());
assert!(analyze("\"unterminated").is_err());
assert!(equivalent("let x = 1 + 2 * 3", "let x = 1+2*3\n").unwrap());
assert!(!equivalent("let x = 1 + 2 * 3", "let x = (1 + 2) * 3").unwrap());
assert!(!equivalent("let x = f \"hi\"", "let x = f\n\"hi\"").unwrap());
assert!(!equivalent("f(1)", "f (1)").unwrap());
```

Also test generics, open ranges, labels, parameter optional/default/type fields, public declaration kinds, invalid trailing input and ordinary/doc/module comment raw spans. Empty input is valid. Inspect AST declarations to choose valid examples whose changed field must compare unequal. Test repeated calls and concurrent calls to the Rust wrapper to detect ownership/global-state mistakes.

- [x] Run the focused tests and record the expected missing-behavior failure. Provision Zig 0.17.0 explicitly if needed and verify its official archive checksum; do not install globally. Copy the minimal unchanged upstream source closure and license from the verified pin. Record provenance and local modifications separately.
- [x] Build a static library directly from the bridge, bypassing upstream optional-runtime build steps. C ABI functions return an owned byte buffer containing structured JSON; an explicit free function releases it. Rust copies/deserializes the payload while it is live and releases it on every path. A null/empty error buffer becomes a validation error, never an unsafe slice. Use a fresh arena for each parse/comparison and never execute macros, imports or semantic compilation.
- [x] Expose `analyze` with complete raw token envelopes. The upstream doc/module token body spans omit delimiters: recover their opening source byte from saved line/column and closing byte from the lexical delimiter rule, verify bounds and delimiters, and test exact source slices. Omit EOF from the token tape. Include ordinary comments and module docs. Reject parse recovery diagnostics even if a tree was recovered.
- [x] Implement structural comparison by reflection or exhaustive traversal. Exclude only values of the exact `ast.Span` type. Compare tagged-union tags, all non-span struct fields, optionals, slices, pointers by contents, float bits and integer/float kind. Do not compare AST debug-print strings. Include `synthetic_block`, doc/attribute/public declaration fields, parameters and generic/implicit-self calls. Repeated source parses must compare equal independently of allocation addresses.
- [x] Make `build.rs` invoke `ZIG` or PATH `zig`, validate the pinned compatible version, pass Cargo host/target information explicitly and reject unsupported targets actionably. Link the generated archive; declare input rerun paths without including target output. Document supported local source-build invocation and the no-runtime-dependency result in `THIRD_PARTY.md`.
- [x] Run `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` with the provisioned toolchain. Inspect vendor checksums against upstream and commit the task. Write the detailed report with red/green evidence, exact checks, build boundary and concerns. Do not start Task 2 before independent review approves this interface.

### Task 2: Implement the width-aware Rust formatter library

**Files:** Create `src/document.rs`, `src/layout.rs`, `tests/formatting.rs`, `tests/fixtures/formatting/`. Modify `src/lib.rs` and `src/oracle.rs` only as needed for tape preservation helpers. `Cargo.toml` changes require justification; no UI or CLI code in this task.

**Interfaces:** Consumes Task 1's analysis/equivalence API. Produces the public API below, used unchanged by the CLI and corpus tests:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent_width: usize,
    pub line_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { indent_width: 2, line_width: 80 }
    }
}

pub fn format(source: &str, options: &FormatOptions) -> Result<String, FormatError>;
```

- [x] Start with expected-output tests for assignment spacing, `do`/`end` indentation and width-driven call/table wrapping. The examples below are the minimal acceptance targets; add targeted syntax hazards from the research notes.

```rust
let defaults = FormatOptions::default();
assert_eq!(format("let x=1+2*3", &defaults).unwrap(), "let x = 1 + 2 * 3\n");
assert_eq!(format("fn f() do\nlet x=1\nx\nend", &defaults).unwrap(),
           "fn f() do\n  let x = 1\n  x\nend\n");
let narrow = FormatOptions { line_width: 24, ..defaults };
let output = format("print(first_argument, second_argument)", &narrow).unwrap();
assert_eq!(output, "print(\n  first_argument,\n  second_argument\n)\n");
assert_eq!(format(&output, &narrow).unwrap(), output);
```

Test a wider setting keeps that call inline; four-space indentation changes nested indentation. Test multi-line tables with commas, binary expressions, match arms without an invented `end`, function/method/generic signatures, type annotations with a lexical colon gap, pipes and short anonymous bodies. Include `f (1)`, `f<T>(x)`, `f <T>(x)`, `for i in 0.. do ... end`, suffix `?`/`!`, and comment-adjacent calls.

- [x] Record a focused red run. Implement a document algebra with text, soft break, hard break, indent, concatenation and group nodes. Render groups flat only when they fit the remaining display columns; use consistent chosen line endings. Keep literal/comment slices borrowed from the original source during layout.
- [x] Construct groups from source tokens, matched punctuation and source-backed regions. Original statement boundaries and line-comment terminators are hard breaks. Indent source `do`/`end` blocks and match-arm bodies. Soft breaks belong at list commas and verified expression continuations. Preserve existing non-semantic blank-line boundaries up to one empty line. Keep an empty source empty and terminate other successful outputs with one final layout newline without modifying an opaque token.
- [x] Normalize ordinary operator/assignment/comma spacing while preserving lexical distinctions and semantic hugging before call parentheses, generic delimiters, labels and range endpoints. Token source text determines spacing classifications; complete source ranges protect comments and literals. Validate options before calling the bridge and return `InvalidOptions` outside specified bounds.
- [x] Validate each candidate by reparsing, exact ordered `(kind, raw_source_bytes)` token tape comparison and complete structural equivalence. If preferred reflow changes syntax, use a documented conservative mode preserving original line-boundary and semantic adjacency decisions and verify it too. If even conservative mode cannot pass, return `Validation` with no candidate. Do not silently emit unsafe text. Ensure reflow decisions and fallback are idempotent, including a second-pass stability check or deterministic algorithm evidence in tests.
- [x] Add exact-byte tests for line/block/doc/module comments, multiline literal closing indentation, escapes, interpolation, backticks, Unicode inside strings/comments, CRLF source and whitespace-only input. Preserve original newlines inside opaque tokens even when layout endings differ. Test malformed input and diagnostics, extreme valid option values, and invalid 0/9 indentation and 19/241 width values.

The raw CR of a CRLF line comment is part of its token. The renderer must append only LF after it, not CRLF, to avoid duplicated CR while keeping the interleaved token/comment tape unchanged. Add record-field doc comments and macro-coordinate behavior to the documented regression cases. Syntax equivalence ignores positions by the user's accepted policy; never claim full behavioral invariance for macros inspecting offsets.
- [x] Run focused red/green cycles while iterating, then the complete suite and Rust lint/style checks once. Commit and report. Every output fixture must include idempotence and oracle-equivalence evidence, not just visual snapshots. Obtain independent task review before CLI work.

### Task 3: Add the CLI, atomic file handling and usage docs

**Files:** Create `src/main.rs`, `src/cli.rs`, `tests/cli.rs`, `README.md`; update Cargo binary metadata and existing provenance notes if necessary. Keep source formatting behavior inside the library.

**Interfaces:** Consumes `format`, `FormatOptions`, `FormatError`, `UPSTREAM_REVISION`. Produces binary `revofmt`. CLI exit codes are 0 success, 1 check differences, 2 usage/I/O/syntax/validation errors. stdout contains only formatted source in print mode. Diagnostic filenames and messages go to stderr.

- [x] Write failing integration tests invoking `env!("CARGO_BIN_EXE_revofmt")` with `std::process::Command` and isolated temporary files. Use actual library behavior, not a stub. A malformed input must fail without stdout and preserve files. Initial assertions include:

```rust
// Invoke stdin formatting with "let x=1".
assert_eq!(output.status.code(), Some(0));
assert_eq!(output.stdout, b"let x = 1\n");
// --check of the same bytes reports a difference without editing.
assert_eq!(check_output.status.code(), Some(1));
assert_eq!(std::fs::read(&path).unwrap(), b"let x=1");
// --write changes them; a subsequent --check succeeds.
assert_eq!(write_output.status.code(), Some(0));
assert_eq!(std::fs::read(&path).unwrap(), b"let x = 1\n");
```

Construct complete test helpers in the test file: unique process/counter temp directory, Drop cleanup, spawn piped stdin/stdout/stderr, and reject errors instead of silently ignoring them. Test help/version and both option overrides. Test `--check --write`, missing option values, out-of-range options, unrecognized flags, stdin/write misuse and multiple print inputs.

- [x] Record the red run, then implement argument handling for stdin/`-`, one print file, multiple check/write files, `--indent-width`, `--line-width`, `--help`, `--version`, and `--` before literal file paths. Keep option defaults in `FormatOptions`.
- [x] Precompute and validate all multi-file write outputs before mutating files. Reject symlinks and nonregular files in write mode. Write a same-directory temporary file with create-new semantics, preserve the original permissions, flush it and atomically replace the destination. Clean temporary files on errors. Unchanged files should not be rewritten. Report completed paths if a later replacement fails. Do not claim multi-file I/O is fully transactional.
- [x] Test that one malformed file in a multi-file write prevents all writes; test missing paths, symlinks, original permissions, and that successful `--write` does not print source. Test `--check` prints changed paths to stderr without source and returns 2 rather than 1 when any input fails.
- [x] Write README commands for source build with Zig 0.17.0/PATH or `ZIG`, CLI usage and Rust library usage. Explain pinned syntax, soft width, opaque literal/comment contents, compile-time-only Zig dependency, initial verified platform and extension of editor integrations later. Include actual commands and a before/after example from tests. No claims of publication or other platform validation.
- [x] Run CLI tests, the complete suite and style/lint checks. Build `cargo build --release` and manually smoke-test stdin, file print/check/write and width override with the release binary. Commit and report actual outcomes. Obtain independent task review.

### Task 4: Add pinned corpus verification and harden regressions

**Files:** Create `tests/corpus.rs`, `tests/fixtures/upstream/`, `tests/fixtures/upstream/PROVENANCE.md`, focused regression fixtures; modify implementation files only to fix reproduced corpus failures. Update README coverage notes if needed.

**Interfaces:** Consumes the reviewed public formatter and options. Existing formatter/CLI contracts remain binding. This task adds broad evidence and fixes demonstrated defects, not new options or editor features.

- [x] Vendor a representative, bounded corpus of upstream `.rv` examples plus extracted self-contained docs snippets from the exact pin, retaining their MIT provenance. Enumerate all chosen fixtures and their parse results. Keep invalid or context-dependent examples in a documented rejection list, not silent test skips. Include demo, pipes, proc, match/control/type syntax and multiline literals where parseable.
- [x] Write a corpus test covering every valid fixture at widths 24, 80 and 120 and indent widths 2 and 4. Assertions use real formatting output, reparsing, exact token tape preservation, complete AST comparison and idempotence. Crate-private oracle assertions may need to live as unit tests with fixture includes; keep public integration coverage in `tests/corpus.rs`. Name each fixture and option combination in failure messages.

```rust
for width in [24, 80, 120] {
    for indent in [2, 4] {
        let options = FormatOptions { line_width: width, indent_width: indent };
        let formatted = format(source, &options).unwrap();
        assert_eq!(format(&formatted, &options).unwrap(), formatted);
        // Internal oracle unit coverage also requires same raw tape and AST.
    }
}
```

- [x] When a fixture exposes a failure, reduce it to a focused failing test before changing implementation. Record the red output and make the smallest root-cause fix; re-run the covering test before broadening. Include malformed source and known whitespace traps as negative controls proving the validator detects unsafe edits.
- [x] Add enough expected-output fixtures to demonstrate real reflow and indentation on the corpus; safety-only unchanged output cannot be presented as broad formatting quality. Identify any safe fallback limitations accurately in README.
- [x] Run the full suite, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, release build and release CLI smoke tests. Use `cargo package --list` to verify vendored build inputs are included without `.tools`, scratch reports or temporary checkout paths. Do not publish. Commit, self-review and report exact corpus counts and limitations. Obtain independent task review followed by one final whole-project review.

## Review and completion

Record task bases, commits, test reports, review findings and fixes in the plan-scoped SDD ledger. Resume the original implementer for substantive findings and independently review the fix range. Research agents' reports also receive independent reviews. Controller changes to design documents must reflect any corrected research facts before implementation relies on them.

Final completion requires fresh local test/lint/build evidence, all task reviews, the whole-project review and an accurate README. Do not merge, push or publish as part of this plan.
