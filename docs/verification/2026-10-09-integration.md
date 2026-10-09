# Revo tooling integration verification, 2026-10-09

The user authorized committing, pushing, merging and removing merged branches
and worktrees after the [upstream refresh](2026-10-09-upstream-refresh.md) and
[architecture and Final Check](2026-10-09-architecture-final-check.md).
Those records describe their earlier, uncommitted stages. All six reviewed
primary checkouts were already on `main` and matched `origin/main`, so this
update integrates directly on `main`.

## Public grammar and pins

The grammar was committed and pushed first. A remote ref check and a fresh
GitHub clone both returned
[`cd1bed33767f7dbfbfbc7882e4a18d5ec229030b`](https://github.com/w0x7y/tree-sitter-revo/commit/cd1bed33767f7dbfbfbc7882e4a18d5ec229030b).
The Zed language extension now selects that available public commit in
`extension.toml`, replacing `f15165b5391656ed3dcce25e18dbfba4320d80ed`.
Its compatibility documentation describes the new structured range nodes,
adjacency checks and intentional interpolation/comment editor recovery.

The formatter frontend and Zed compiler audit remain on the verified upstream
`e94e6d89ddaabb3249b38c1b10df87c700d1e8dc`, updated from
`b571298b6fc95bc863548f118354c8d077792f6f` during the refresh.
Automatic formatter/server downloads retain their existing available release
assets and checksums. This integration does not publish packages or releases.

## Fresh tests before integration

Commands ran from the named repository, with:

```sh
export ZIG=/home/idan/GitRepo/revo-formatter/.tools/zig-x86_64-linux-0.17.0/zig
export REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt
export REVOFMT_CURRENT_SYNTAX=1
```

**Formatter:** `cargo test --all-targets`, `cargo test --release --all-targets`
and `cargo fmt --check` pass. Both suites pass 100 tests (81 library, one
binary, 18 CLI), including preservation, idempotence and resource-limit cases.
`REVO_BIN=/home/idan/GitRepo/revo/zig-out/bin/revo scripts/check-upstream`
passes 20 programs, 80 compiler/behavior/idempotence checks, 14 syntax
rejections and one compiler-only rejection against the exact e94 compiler.

**Grammar:** `tree-sitter generate`, `tree-sitter test --json-summary`,
`python3 test/ast/check_generic_calls.py . queries/highlights.scm`,
`python3 test/ast/check_expressions.py .`,
`python3 test/ast/check_upstream.py .`, and `npm test` pass. Results are 188
corpus cases, 18/21 generic cases, 25 expression cases, 48 valid whole
programs, two intentional editor recoveries, seven invalid loops and six
native binding tests. Regeneration leaves the committed generated source
unchanged. The metadata caller also passes:

```sh
npm exec --yes --package=typescript@5.9.3 -- tsc --noEmit --strict \
  --module nodenext --moduleResolution nodenext test/types/node-metadata.mts
```

**Formatter adapters:** `scripts/verify` passes in each repository with the
shared environment above. Neovim passes 34 formatting and 15 installer tests;
VS Code passes 56 tests; the Zed formatter passes 15 CLI/metadata checks,
79 Node tests, 13 Rust tests, formatting, Clippy and four server runtime digests.

**Zed language extension:** `cargo fmt --check`,
`cargo clippy --locked -- -D warnings`, `cargo test --locked` (zero unit tests)
and `cargo build --locked --release --target wasm32-wasip2` pass. The following
checks use a clean fresh clone at the exact new public grammar pin:

```sh
python3 tests/check_scopes.py /path/to/public-grammar
python3 tests/check_highlights.py /path/to/public-grammar
python3 tests/check_grammar.py /path/to/public-grammar
python3 tests/check_grammar_support.py /path/to/public-grammar
python3 tests/check_grammar_patch.py /path/to/public-grammar
python3 tests/check_compiler_syntax.py /path/to/public-grammar /home/idan/GitRepo/revo
```

All pass: 188 pinned corpus cases, queries/scopes, canonical AST and parser
ownership/failure checks, 55 compiler fixtures and one documented invalid
fixture. The independent historical patch check also passes its own 132-case
corpus; it intentionally reproduces the original Codeberg-base patch rather
than replacing the current public grammar verification.

Logs for this integration are under `/tmp/revo-integration-20261009-1zoVQj`.
The preceding Final Check record contains the additional host, package,
checksum, dependency-audit and upstream-suite evidence, including checks not
available on this Linux host. This follow-up adds no runtime source changes
beyond the already reviewed tree; it updates the available grammar pin and
current documentation.
