# Contributor instructions

`revofmt` is a Rust library and CLI backed by a statically linked, pinned Revo
frontend. Read [README.md](README.md) for build and test commands,
[CONTEXT.md](CONTEXT.md) for domain terms, and
[docs/architecture.md](docs/architecture.md) before changing module boundaries.

## Preserve these contracts

- Every returned result must preserve the exact interleaved token/comment tape,
  preserve the complete syntax tree modulo source coordinates, and be idempotent.
  Keep literals and comments opaque. Use the real preservation interface when
  testing layout; expected output alone does not establish safety.
- Keep parser-sensitive call, generic, label and range adjacency. Short headers,
  return tables and generic lists should stay compact when they fit independently
  of their bodies or following arguments. Width is a soft target.
- Apply resource admission before recursive parsing, traversal or layout,
  including generated candidates. Read the
  [input admission policy](docs/verification/input-limits.md) before changing
  grammar-sensitive limits or adding recursive paths.
- Preserve CLI exit codes and prevalidation of the complete write batch. Each
  file replacement is atomic; a batch is not a single transaction.
- `vendor/revo/` is unchanged upstream source. Keep its revision, checksums and
  license together. Toolchain or upstream pin changes need explicit scope and
  fresh build, syntax, ABI and resource-limit verification. Builds never download
  tools or source. The verified platform is native Linux x86_64 GNU with exact
  Zig 0.17.0.

## Making changes

Formatting regressions belong in `src/tests/formatting.rs`; corpus checks are in
`src/tests/corpus.rs`; actual CLI process tests are in `tests/cli.rs`.
For a formatting fix, add a focused failing case with literal expected output,
preservation and idempotence checks, then fix its owning module. Avoid importing
private Rust implementations into separate integration test binaries.

Use the [full verification commands](README.md#development-and-verification)
before completing source changes. Run checksum manifests from their own
directories. If a reviewed golden or corpus provenance changes, regenerate its
manifest without changing upstream fixture bytes or license notices.

Update the current guides when behavior or interfaces change. Historical plans,
research and dated review results are evidence of their recorded stages; their
old test counts, line references and workspace paths are not current instructions.
The [documentation index](docs/README.md) identifies current references.
