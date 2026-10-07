# Revo syntax stress verification, 2026-10-07

The user requested documentation research, a broad Revo test file, and a check
for formatter damage. Primary-source findings are in the
[research note](../../research/2026-10-07-revo-syntax-stress.md). The handwritten
[stress source](../../tests/fixtures/stress/syntax.rv), its
[reviewed default output](../../tests/fixtures/stress/syntax.expected.rv), and
[reproduction instructions](../../tests/fixtures/stress/README.md) remain in the
repository.

No returned syntax corruption, changed token/comment bytes, crash, or
non-idempotent output was found in this run. The actual preservation interface
checked 12,912 valid input/option combinations in each of debug and release:
the whole file, its 51 sections including the module comment, and 36 expressions
in six surrounding constructs, plus the user's exact
[file-read match](../../tests/fixtures/stress/match-file-read.rv). The latter
combines `fs.open(...)?:read()`, `|` arms, a guarded arrow on the next line,
a typed table pattern and interpolation. Its default layout is checked against
literal expected output. Inputs used LF/CRLF, six widths and four indent
settings. Fifteen malformed near misses passed 360 syntax-rejection checks.

The first combined draft exceeded the existing parser admission score and
returned a validation error with no CLI output. Repeated examples were trimmed
until the complete file was admitted; resource limits were not raised.
Invented syntax rejected by the pinned parser was retained as explicit negative
tests. This includes a labeled `do` expression immediately followed by a pipe.
The current upstream parser also differs from the pinned revision, particularly
in spacing before range dots; the research note records the precise revisions.

The full file uses conservative layout, so its reviewed result mostly changes
indentation. Isolated sections and composed expressions check formatting
independently. This run proves preservation for these examples, not exhaustive
grammar coverage, uniform visual quality, or runtime behavioral equivalence.
Programs were never executed and imports were never loaded. Source-coordinate
changes visible to procedural macros remain the documented limitation.

## Completed checks

Native Linux x86_64 GNU, exact Zig 0.17.0 from the existing local toolchain:

- `cargo test --all-targets`: 77 library, 1 binary and 17 CLI tests passed.
- `cargo test --release --all-targets`: the same 95 tests passed.
- `cargo test --doc`: passed, with no doctests present.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- Zig formatting check for `bridge.zig` and `bridge/*.zig`: passed.
- README's filtered ReleaseSafe Zig bridge/input-limit/indexed suite: 10 tests passed.
- `cargo build --release`: passed.
- Vendor and upstream fixture checksum manifests: passed from their own directories.
- Rebuilt release CLI output equals the reviewed file; a second pass is identical;
  `--check` on the reviewed output exits successfully without diagnostics.
- Relative documentation links and whitespace: passed.

The formatter implementation, frontend/toolchain pins, vendor bytes, upstream
fixture bytes and licenses were unchanged. Changes add synthetic fixtures,
library/CLI tests and documentation. Editor behavior was not changed.
