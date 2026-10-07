# Contributor instructions

`revofmt` is a Rust library and CLI backed by a statically linked, pinned Revo
frontend. Editor adapters live in [dedicated repositories](docs/editors.md).
Read [README.md](README.md) for build and test commands,
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
  grammar-sensitive limits or adding recursive paths. Counters are global across
  decoded fragments; test mixed forms on 2 MiB debug and release threads.
- The source collector owns actual block envelopes from parsed provenance.
  Revo also accepts `do` and `end` as names. Preserve real source blocks under
  generated wrappers while keeping synthetic hints and opaque interiors excluded.
- Preserve CLI exit codes and prevalidation of the complete write batch. Each
  file replacement is atomic; a batch is not a single transaction.
- `vendor/revo/` is unchanged upstream source. Keep its revision, checksums and
  license together. Toolchain or upstream pin changes need explicit scope and
  fresh build, syntax, ABI and resource-limit verification. Builds never download
  tools or source. The verified platform is native Linux x86_64 GNU with exact
  Zig 0.17.0.

## Making changes

Formatting regressions belong in `src/tests/formatting.rs`; admission regressions
in `src/tests/input_limits.rs`; corpus checks in `src/tests/corpus.rs`; actual CLI
process tests in `tests/cli.rs`.
For a formatting fix, add a focused failing case with literal expected output,
preservation and idempotence checks, then fix its owning module. Avoid importing
private Rust implementations into separate integration test binaries.

Use the [full verification commands](README.md#development-and-verification)
before completing source changes. Run checksum manifests from their own
directories. If a reviewed golden or corpus provenance changes, regenerate its
manifest without changing upstream fixture bytes or license notices.

The [editor integration guide](docs/editors.md) defines the public stdin/stdout
contract and links to the adapter repositories. Keep editor source, packaging,
transport and lifecycle tests in their owning repository. When changing CLI
behavior consumed by adapters, run their documented checks against the rebuilt
formatter. Keep personal editor settings unchanged unless the user asks.

Update the current guides when behavior or interfaces change. Historical plans,
research and dated review results are evidence of their recorded stages; their
old test counts, line references and workspace paths are not current instructions.
The [documentation index](docs/README.md) identifies current references.
For documentation-only changes, check examples against their owning interface,
relative links, and whitespace. Preserve vendor source, corpus input bytes and
license notices.
