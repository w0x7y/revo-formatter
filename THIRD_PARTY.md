# Pinned Revo frontend

`vendor/revo/` contains unchanged files from https://github.com/if-not-nil/revo
at revision `1060fcf44a16a185263b3cb244cca886ac017acc`.
Paths under that directory match upstream paths exactly. `REVISION` and
`SHA256SUMS` are local provenance metadata. Revo is MIT licensed; its complete
notice is retained in `vendor/revo/LICENSE.txt`. The formatter's original code
is separately covered by the root `LICENSE`.

`tests/fixtures/upstream/` contains unchanged examples and documentation extracts
from the earlier revision
`b571298b6fc95bc863548f118354c8d077792f6f`, plus locally reviewed formatted
expected outputs. These historical inputs remain unchanged and are tested
against the current frontend pin.
Its [PROVENANCE.md](tests/fixtures/upstream/PROVENANCE.md) enumerates every
selected input with exact upstream paths and line ranges. The corpus retains
its own unchanged upstream MIT notice in `LICENSE.txt` and file checksums in
`SHA256SUMS`.

The 22 Zig files are the relative-file import closure of `Parser.zig`,
`Lexer.zig`, `ast.zig`, `diagnostic.zig`, and `type_syntax.zig`. Zig resolves
imports appearing in unused declarations and tests, so source files for those
imports must exist. Their runtime operations are not used by this bridge.
`bridge/frontend.zig` calls only the lexer and pure parser. It never calls
pipeline parsing, loads imports, expands macros, performs semantic analysis,
or invokes a Revo runtime. No upstream source modifications or stub runtime
modules are used. The separate `bridge/` modules implement owned JSON results,
source metadata, and exhaustive structural comparison.

The refreshed closure still contains 22 files. Upstream's `global_const` AST
variant participates in the bridge's existing exhaustive enum comparison and
generic source traversal; neither needs a special declaration case. New
runtime-facing imports in upstream diagnostics remain unused by pure parsing.
The bridge build still needs only Zig's standard library and libc, with no new
Cargo or Zig package dependencies. Filtered bridge tests and the native debug
and release suites verify this build boundary.

The pin moved from `e94e6d89ddaabb3249b38c1b10df87c700d1e8dc` to
`f0034ab75aaf49d65bc1b4769987f99380383fcb` without file changes: upstream
changed only its README and default build features, and all 22 vendored files
and `LICENSE.txt` match `SHA256SUMS` at both revisions.

The pin then moved from `f0034ab75aaf49d65bc1b4769987f99380383fcb` to
`9c46813e054381b080ea8a3ea418247dd5912596`, again without file changes:
upstream changed only its Nix flake, which now uses Zig 0.17. The vendored
files and `LICENSE.txt` still match `SHA256SUMS`.

The pin then moved from `9c46813e054381b080ea8a3ea418247dd5912596` to
`c9ff2105656803359867a6f13213330ee672d898`, again without file changes:
upstream changed only a link in its README. The vendored files and
`LICENSE.txt` still match `SHA256SUMS`.

The pin then moved from `c9ff2105656803359867a6f13213330ee672d898` to
`1f95869439fe41da49aab0cce7fa94134ec5ddd4`, again without file changes:
upstream changed only its Nix flake, which now enables the LSP build feature.
The vendored files and `LICENSE.txt` still match `SHA256SUMS`.

The pin then moved from `1f95869439fe41da49aab0cce7fa94134ec5ddd4` to
`d2f16441c1554fec50d0098434223850bc4a1868`, again without file changes:
upstream changed only its `build.zig` default features, which now include LSP
on native targets and drop it from wasm. The vendored files and `LICENSE.txt`
still match `SHA256SUMS`.

The pin then moved from `d2f16441c1554fec50d0098434223850bc4a1868` to
`482ef26f80f7b7b88e3c4b74d20b914a00e6e5f6`, again without file changes:
upstream added `dataframe.sort()` to its runtime library in
`src/baselib/base.rv` and `src/baselib/dataframe.zig`, which are not vendored.
The vendored files and `LICENSE.txt` still match `SHA256SUMS`.

The pin then moved from `482ef26f80f7b7b88e3c4b74d20b914a00e6e5f6` to
`1060fcf44a16a185263b3cb244cca886ac017acc`, again without file changes:
upstream added `dataframe.row()` and `dataframe.filter()` to its runtime
library and fixed its `argparse` help output and HTTP keepalive, all outside the
vendored files. The vendored files and `LICENSE.txt` still match `SHA256SUMS`.

## Source builds

Install exact stable Zig **0.17.0** yourself, then run:

```sh
export ZIG=/absolute/path/to/zig-0.17.0/zig
cargo build --release
(cd vendor/revo && sha256sum --check SHA256SUMS)
(cd tests/fixtures/upstream && sha256sum --check SHA256SUMS)
```

`ZIG` selects the compiler; if unset, the build script tries `zig` on PATH.
The script accepts only version output `0.17.0`. It never downloads source or
tools and does not know the developer's local `.tools/` directory.
The official Linux x86_64 archive is
`https://ziglang.org/download/0.17.0/zig-x86_64-linux-0.17.0.tar.xz`, SHA-256
`1cbe9df9f27e6b78d14ccbca43b6703a404ef79ef1c463de901d7f088d4e2026`.
The development toolchain was explicitly provisioned and checksum verified
before this implementation task.

Native `x86_64-unknown-linux-gnu` is the verified and supported source-build
host/target pair. `build.rs` checks both Cargo `HOST` and `TARGET`, passes
`x86_64-linux-gnu` and baseline CPU settings explicitly to Zig, and rejects
other combinations with an actionable diagnostic. Other platforms require
separate linking and ABI validation.

The bridge is built directly as a position-independent ReleaseSafe static
archive, including Zig compiler-rt. It bypasses upstream `build.zig` and its
optional dependencies. Resulting binaries have no separate Zig or Revo runtime
dependency; normal Linux C/system libraries still apply. Build caches and
archives are placed under Cargo's output directory, outside tracked sources.

Run the complete Rust and filtered local Zig suites from the
[development checks](README.md#development-and-verification). See the
[current architecture](docs/architecture.md) for contributor guidance. Verify
manifests from the directories shown; their paths are directory-relative.

## Syntax validation policy

Comparison ignores only the exact upstream `ast.Span` type, including nested
parameter/type coordinates. It compares all other fields, tagged unions,
optional values, slices and pointed-to contents, with floats compared by bits.
This establishes syntax equivalence modulo coordinates. Procedural macros
that inspect positions can observe formatting-induced coordinate changes;
the bridge never executes those macros. The Rust formatter separately checks
one interleaved ordered sequence of raw tokens and comments for exact byte
preservation before returning a result.
