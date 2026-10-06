# Revo formatter v0 design

Historical approved design/implementation plan; implementation and its review gates are complete. Original interface sketches and stage-specific instructions below are retained as design history. Use the [current architecture](../../architecture.md) for delivered module boundaries and the [final source check](../../verification/2026-10-06-final-check.md) for verification.

## User decisions

- Build here in Rust.
- Ship a CLI and reusable Rust library first; editor integration comes later.
- Include line-width reflow in the first version.
- Expose a few options, with defaults informed by Revo's documentation.
- Print formatted code by default; provide `--write` and `--check`.
- Reuse a pinned upstream Zig parser for validation if the build and licensing work.
- Delegate implementation tasks and independently review every task.
- Allow formatting procedural macros; preserve syntax and treat source-position
  changes as normal formatting effects.

## Scope and defaults

The executable and crate are named `revofmt`. The public API is
`format(source: &str, options: &FormatOptions) -> Result<String, FormatError>`.
`FormatOptions` has `indent_width` and `line_width`. Defaults are two spaces
and 80 columns. Revo's editor documentation explicitly recommends two-space
indentation; it does not specify a line width, so 80 is our initial choice.
Indent width accepts 1 through 8; line width accepts 20 through 240.

Width is a target, not a guarantee. Long literal/comment contents and syntax
that cannot safely break may exceed it. Preserve all literal and comment
bytes, including documentation comments, quasiquotes, interpolation and
multiline-string internal indentation. The formatter does not sort imports,
rename identifiers, alter number spelling, change quotes, or execute Revo.

Support the syntax accepted by upstream revision
`b571298b6fc95bc863548f118354c8d077792f6f`. Unknown or invalid input returns
a diagnostic and no formatted output. Parse validity does not imply type
correctness; formatting must also work for syntactically valid incomplete
projects without resolving imports.

## Architecture

The Rust library owns formatting policy, document layout, configuration,
diagnostics and the CLI. A small statically linked C ABI bridge calls the
vendored upstream lexer and parser. The bridge exposes lossless source
token ranges, syntax layout metadata where useful, and structural comparison
of parsed trees. Use the compiler parser as the syntax authority; the
existing tree-sitter grammar is not authoritative for this implementation.

Source flows through parsing and token analysis, Rust layout, and validation
of the candidate against the original. Structural comparison excludes source
spans and includes every semantic field. The upstream AST pretty-printer is
not a suitable comparison mechanism because it omits fields. Ordinary
comments also require a separate ordered exact-byte preservation check.
Non-whitespace token spellings must stay unchanged.

The guarantee is syntax equivalence modulo source coordinates, rather than
identical results for programs that inspect positions. Revo proc macros can
observe nested parameter/type spans; formatting may therefore affect a macro
that intentionally reads offsets, lines or columns. The user explicitly
accepted this policy. Document it in the public API and README without
executing macros to validate formatting.

The formatter knows which token gaps encode calls, generic calls, range
bounds and labels. Preserve those distinctions while spacing and grouping
expressions. Candidate reflow must parse successfully and retain the same
tree. If a proposed line break changes structure, choose a more conservative
layout and validate it too. Never return a candidate that fails validation.
An unchanged or conservative result is allowed where a wider line is needed
to preserve syntax. Formatting the output again must produce identical bytes.

Use a small document representation for text, soft breaks, hard breaks,
indentation and groups. Lay out delimited lists, tables and function
arguments according to the available width. Preserve statement boundaries;
indent `do`/`end` blocks and match arms. Keep comments attached to their source
location and force line comments to terminate their line. Leave literal
contents opaque rather than formatting interpolation bodies independently.
Preserve input line-ending convention for layout breaks and emit one final
newline for nonempty source. Empty source stays empty. Preserve blank lines
between statements, capped at one empty line where it does not affect syntax.
The token/comment comparison is one interleaved sequence so comments cannot
move across code tokens. A CRLF line comment's raw token already includes CR;
emit only the missing LF after that token instead of adding another CR.

## Build and provenance

Vendor the smallest upstream source closure needed by the pure frontend,
with its exact revision, unchanged upstream files, and MIT license notice.
Keep the bridge separate from vendored files. Rust source builds require
the compatible Zig toolchain; release binaries contain the parser and do not
require a separately installed Revo or Zig executable at runtime.
Pin the frontend build to the stable Zig 0.17.0 compiler and standard library,
because escape decoding uses Zig's standard library. Record the official
host-archive checksum in the build/provenance documentation.
The first implementation task must compile and exercise this bridge before
the printer is implemented. Build scripts must not download tools or source
implicitly. Report an actionable error when Zig is missing or incompatible.
Linux on the current machine is the initial verified platform. Cross-platform
release claims require validation on those platforms.

## CLI

- `revofmt` or `revofmt -` reads stdin and prints formatted source.
- `revofmt FILE` prints that file's formatted source.
- `revofmt --write FILE...` formats files in place.
- `revofmt --check FILE...` reports files requiring changes without editing.
- `--indent-width N` and `--line-width N` override the defaults.
- `--help` and `--version` describe usage and the formatter version.

Print-source mode accepts one input. `--write` and `--check` are mutually
exclusive; write mode requires file paths and rejects stdin. Use exit code
0 for success, 1 for check differences, and 2 for arguments, I/O, parse or
validation errors. Diagnostics go to stderr. Do not print partial source
when formatting fails. For multi-file writes, compute and validate every
result before starting replacements. Each replacement must be atomic and
preserve file permissions. Reject symbolic links in write mode. A later I/O
failure can leave earlier successful replacements in place and must report
that fact. Directory discovery and configuration files are outside v0 scope.

## Verification and reviews

Each implementation task has tests written before its behavior and an
independent review for requirement compliance and code quality. Test the
bridge on lexical/parse errors and structure changes involving precedence,
bare calls, generic calls, range bounds, labels, defaults and type syntax.
Test formatting with expected output, both indentation options and width
options, opaque literal/comment preservation, deterministic reflow and
idempotence. Exercise actual CLI processes on temporary files and stdin,
including invalid input, permissions and check exit codes.

Keep a pinned, attributed corpus from upstream examples and docs. Only
fixtures confirmed parseable by the pinned frontend can be counted as valid
formatting coverage. Every valid fixture must still parse, preserve its tree
and token contents, and be idempotent at multiple widths. Record rejected
examples explicitly rather than silently skipping them. Final checks include
`cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
a release build, CLI smoke tests and a whole-project independent review.

## Implementation boundaries

1. Upstream frontend bridge, provenance, Cargo build and Rust oracle API.
2. Rust document layout and formatter library with focused regression tests.
3. CLI, documentation and end-to-end file/stdin tests.
4. Pinned corpus and regression hardening, followed by final independent review.

These tasks are sequential: each builds on reviewed interfaces from the
previous task. No publication, remote repository creation or editor extension
is part of this work.
