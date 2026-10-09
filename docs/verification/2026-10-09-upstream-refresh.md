# Revo upstream refresh, 2026-10-09

The formatter frontend and Zed compiler audit now use upstream
`e94e6d89ddaabb3249b38c1b10df87c700d1e8dc`, verified with
`git ls-remote https://github.com/if-not-nil/revo.git HEAD refs/heads/main`.
Both refs returned that full hash. `git rev-list --count
b571298b6fc95bc863548f118354c8d077792f6f..HEAD` returned 38.
All work is local and uncommitted; no packages, releases, pushes or PRs were
published.

## Repository state and pins

All repositories are under `/home/idan/GitRepo/`. The missing
`tree-sitter-revo` and upstream `revo` were cloned there. The six tooling repos
started clean except for the existing untracked `revofmt-zed/graft/`, which
was preserved. Each repository was checked for contributor instructions,
working-tree changes, manifests, lockfiles and revision pins before editing.
The formatter's `AGENTS.md`, current guides and verification commands govern
its changes; the other repositories have no contributor instruction files.

- `revo-formatter` started at `f0cff5c2b0a82931ad57a2a4a7f4034d597f6d75`.
  Its frontend pin and public `UPSTREAM_REVISION` changed from
  `b571298b6fc95bc863548f118354c8d077792f6f` to
  `e94e6d89ddaabb3249b38c1b10df87c700d1e8dc`.
- `tree-sitter-revo` started at published commit
  `f15165b5391656ed3dcce25e18dbfba4320d80ed`. The updated grammar is local
  source, with no invented replacement commit hash.
- `revo-zed-extension` started at
  `5385c5add959b1345265d3b24108c4b64aa90c66`. Its compiler audit pin changed
  from `b571298b6fc95bc863548f118354c8d077792f6f` to
  `e94e6d89ddaabb3249b38c1b10df87c700d1e8dc`. Its public grammar dependency
  remains `f15165b5391656ed3dcce25e18dbfba4320d80ed`.
- `revofmt.nvim` started at `e3af3a00995ffe8008279d6e422892737425a6cd`.
  The published formatter pin remains v0.1.1, source revision
  `1eb68eb07e5968cccc8b25ccad1357b8344b78c1`, SHA-256
  `bcc31238dff6b533c10a23c71e110d6e3211bbedb22dcde4d13f19b986e8937c`.
- `revofmt-vscode` started at
  `ad9ff9294886c13c82e1a9f252383699bfcf7dc0`. Its packaging/tool dependencies
  and version remain unchanged.
- `revofmt-zed` started at `6431ca7074defa54d7acfcaad96d3a63a91e1f0d`.
  Its published server v0.2.0 URL and digests remain unchanged.

No tooling repo was left entirely unchanged: each needed source, regression
tests or current audit documentation. The upstream checkout has no source
changes. The three formatter adapters' runtime code, all Cargo/npm lockfiles,
and published dependency URLs, versions and digests remain unchanged.

Dependency review found no required version changes. The formatter retains
exact Zig 0.17.0, its 22-file frontend import closure, and its existing Cargo
dependencies. The grammar retains Tree-sitter CLI 0.26.9 and its Node binding
dependencies. Both Rust Zed integrations retain `zed_extension_api = 0.7.0`;
the formatting adapter also retains `sha2 = 0.10.9`. VS Code retains
`@vscode/vsce = 4.0.0`. Adapter execution still uses the same stdin/stdout CLI
contract.

## Changes and authority

The [range adjacency change](https://github.com/if-not-nil/revo/commit/71115dea59391f2fbb1e5e79226157aea3a6ffd5)
rejects gaps before loop-range start and step dots. A gap after dots can still
start the body of an open-ended range. The
[interpolation change](https://github.com/if-not-nil/revo/commit/e94e6d89ddaabb3249b38c1b10df87c700d1e8dc)
rejects unknown trailing modes, while accepting `:v`, `:?`, `:p` and the lone
atom `"#{:d}"`. Upstream `Parser.zig` and `src/lang/tests/strings.zig` supply
the regression boundaries. The old formatter accepted the newly invalid
programs; no case demonstrated the formatter introducing invalid spacing.

The new rejection regressions failed against the old frontend, then passed
after replacing the vendor files. Every one of the 22 Zig files and the
license matches `git show e94e6d89ddaabb3249b38c1b10df87c700d1e8dc:<path>`
byte-for-byte. Updated vendor checksums pass. The unchanged historical
upstream corpus retains its own baseline, original bytes, license and
checksum manifest.

The existing bridge traverses AST fields generically and compares enums
exhaustively, including `global_const`. The new upstream runtime-facing
diagnostic imports remain unused by pure parsing. Native debug/release,
filtered Zig tests and the offline package build verify that `bridge/*.zig`,
`build.rs`, Cargo dependencies and ABI need no implementation changes.

The valid synthetic stress corpus had two now-invalid spaced loop ranges.
Those boundaries are rejection tests; the valid corpus now uses open-ended
range gaps before loop bodies, with reviewed output updated together.
[scripts/check-upstream](../../scripts/check-upstream) checks complete
programs with the real compiler, including execution output before/after
formatting, compile acceptance and idempotence at four widths.

The grammar enforces whitespace adjacency before loop dots, keeps the
open-range body boundary, and adds omitted/computed/reverse slice bounds.
Independent review found numeric-prefix truncation in the legacy range
scanner; complete programs with underscore, exponent and base-prefixed
bounds now have regressions and defer to the ordinary number grammar.
Generated parser files were regenerated reproducibly. The grammar's own
query adds a `..` delimiter capture.

The three formatter adapters add real-CLI tests for valid interpolation and
fixed points, plus current-syntax rejection with no buffer/LSP edits. Set
`REVOFMT_CURRENT_SYNTAX=1` for the rejection checks: published-binary CI
intentionally uses older, available formatter releases. Neovim's installer
tests now use only controlled tool executables so an installed user formatter
cannot invalidate a missing-formatter test.

## Changed files

In `revo-formatter`: `README.md`, `THIRD_PARTY.md`, `docs/README.md`,
`docs/formatter.md`, this report, `src/lib.rs`, `src/tests/formatting.rs`,
`tests/cli.rs`, `scripts/check-upstream`, and the stress corpus's `README.md`,
`syntax.rv` and `syntax.expected.rv`. Vendor changes are `REVISION`,
`SHA256SUMS`, and unchanged upstream copies of `Parser.zig`, `ast.zig`,
`diagnostic.zig`, `import_types.zig`, `macro_common.zig`, `macro_proc.zig`,
`pipeline.zig`, `scope_graph.zig`, `semantic.zig`, `test_helpers.zig`,
`type_syntax.zig`, `compiler/{bindings,control,locals,root,types}.zig`,
`ir/{opt,root}.zig` and `pipeline/import_scan.zig`.

In `tree-sitter-revo`: `grammar.js`, `src/scanner.c`, generated
`src/{parser.c,grammar.json,node-types.json}`, `queries/highlights.scm`,
`test/corpus/upstream-current.test`, `test/ast/check_upstream.py`,
`bindings/node/binding_test.js` and `README.md`.

In `revo-zed-extension`: `tests/compatibility.toml` and
`docs/grammar-continuation.md`. Published queries stay compatible with its
existing grammar pin.

In `revofmt.nvim`: `README.md`, `tests/run.lua` and `tests/install.lua`.
In `revofmt-vscode`: `README.md` and `tests/provider.test.js`.
In `revofmt-zed`: `README.md` and `tests/lsp.test.cjs`.

## Exact verification commands

Commands below ran from the indicated repository. Shared settings were:

```sh
export ZIG=/home/idan/GitRepo/revo-formatter/.tools/zig-x86_64-linux-0.17.0/zig
export REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt
export REVOFMT_CURRENT_SYNTAX=1
```

From `revo-formatter`:

```sh
cargo test --lib upstream_ -- --nocapture
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
REVO_BIN=/home/idan/GitRepo/revo/zig-out/bin/revo scripts/check-upstream
REVO_BIN=/home/idan/GitRepo/revo/zig-out/lsp/bin/revo scripts/check-upstream
cargo package --offline --allow-dirty
git diff --check
```

Results: 99 tests pass in both debug and release, comprising 80 library,
1 binary and 18 CLI tests. The corpus includes 12,912 stress
preservation/idempotence combinations. Ten filtered Zig tests pass. Doc tests
run successfully with zero examples. Clippy, Rust/Zig formatting, release
build, both checksum manifests and offline package build pass. Each
compiler-backed run passes 20 valid programs, 80 compile/behavior/idempotence
checks, 14 syntax rejections and one compiler-only rejection. Before the
refresh, the focused command had two expected failures and one passing valid
boundary test. The first full run after refreshing the vendor identified the
two obsolete valid stress cases; both final full suites pass after updating
that handwritten corpus.

From upstream `revo`:

```sh
"$ZIG" build -Doptimize=ReleaseSafe -Dfeatures= -Dglibc -j2
"$ZIG" build test -Doptimize=ReleaseSafe -Dfeatures= -Dglibc -j2
"$ZIG" build test -Doptimize=ReleaseSafe -Dfeatures=regex -Dglibc -j2
"$ZIG" build -Doptimize=ReleaseSafe -Dfeatures=lsp,regex -Dglibc -j2 --prefix zig-out-lsp
mv zig-out-lsp zig-out/lsp
./zig-out/lsp/bin/revo version
```

Both compiler builds pass and identify `revo 0.1.2 (e94e6d8)`. The
feature-disabled upstream suite reports 701 passes, 8 skips and one failure:
`baselib.specs.test.moduleDoc finds table docs in the single file` expects
regex documentation while regex is disabled. The regex-enabled suite passes
with exit 0. No upstream source was altered to hide that configuration
failure. FFI, isocline and mimalloc configurations were not exercised.

From `tree-sitter-revo`:

```sh
tree-sitter generate
tree-sitter test --json-summary
python3 test/ast/check_upstream.py .
python3 test/ast/check_expressions.py .
python3 test/ast/check_generic_calls.py . queries/highlights.scm
npm ci --no-audit --no-fund
node-gyp rebuild
npm test
tree-sitter build --output /tmp/revo-local-audit.so
tree-sitter build --wasm --output /tmp/revo-local-audit.wasm
git diff --check
```

All pass: 173 corpus cases; 33 valid complete programs, 2 documented editor
recoveries and 6 invalid loop programs; 25 expression checks; 18 positive and
21 negative generic checks; 5 freshly rebuilt native-binding/query/incremental
tests. Native and Wasm builds pass. Regeneration reproduces the final generated files.

Supplementary independent review used temporary inline Python/Node probes:
237 compiler candidates included 171 compiler-accepted grammar programs with
zero parser failures after the numeric fix; 184 compiler-valid formatter
programs produced 552 successful compile/idempotence checks at widths
20/24/80; 1,426 insert/delete edits produced identical fresh and incremental
trees. The permanent scripts above retain the core boundary regressions.

## Editor verification

With the shared settings above, `scripts/verify` passes from each of
`revofmt.nvim`, `revofmt-vscode` and `revofmt-zed`. Results are respectively
34 buffer plus 13 installer tests; 54 provider/transport tests; and 15 Python,
79 Node and 9 Rust tests, plus formatting, Clippy and four runtime digest
checks. All current-syntax rejection checks are enabled in these results.

The preserved pre-refresh formatter at
`/tmp/revo-editor-audit-jl_c5279/revofmt-before` also passes the default checks
when `REVOFMT_CURRENT_SYNTAX` is unset: 33+13 Neovim tests, 53 VS Code tests
with one skip, and 15+78+9 Zed tests with one skip. Enabling the current-syntax
flag against that old binary produces the expected rejection-test failures.

Additional commands from `revofmt-vscode`:

```sh
REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt npm run test:host
npm ci && npm run package
```

Both pass. VS Code 1.140.0 verifies both file suffixes, activation, unsaved
formatting, idempotence and unchanged disk content. The VSIX builds and npm
audit reports zero vulnerabilities.

From `revo-zed-extension`:

```sh
cargo fmt --check && cargo clippy --locked -- -D warnings && cargo test --locked && cargo build --locked --release --target wasm32-wasip2
python3 tests/check_scopes.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_highlights.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_grammar.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_grammar_support.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_grammar_patch.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_compiler_syntax.py /tmp/revo-editor-audit-jl_c5279/grammar /home/idan/GitRepo/revo
```

All pass. The temporary grammar checkout is the exact published
`f15165b5391656ed3dcce25e18dbfba4320d80ed` pin: 132 corpus cases, existing
AST/query checks, 55 supported current compiler fixtures and one recorded
invalid fixture. The Rust adapter has zero unit tests; its build, Clippy and
actual host launch are the substantive checks. A separately rebuilt latest
local grammar also passes `grammar_support.run_checks()` and
`check_compiler_syntax.verify()`: 173 corpus cases, all existing Zed queries,
and the same 55+1 fixtures. Its full output is in
`/tmp/revo-editor-audit-jl_c5279/local-grammar-final.log`.

From `revofmt-zed`:

```sh
cargo build --release --target wasm32-wasip1 --locked
cargo build --release --target wasm32-wasip2 --locked
```

Both builds pass. The documented wasip1 artifact was checked; fresh wasip2
adapters were built separately for loading in the current Zed component host.

Native Zed checks used temporary user data, workspace, settings and freshly
compiled published grammar Wasm, with fresh wasip2 adapters and the downloaded
existing pinned server:

```sh
python3 /tmp/revo-editor-audit-jl_c5279/check-zed-host.py
python3 /tmp/revo-editor-audit-jl_c5279/check-revolt.py
```

Both pass. Zed 1.22.0 verifies unsaved Unicode/literal/interpolation formatting,
a fixed point, one undo, three invalid current-upstream programs left untouched,
and unchanged backing files. The final formatting run has both current servers
active. The current upstream Revolt check verifies initialization, hover,
definition, a two-site rename, completion, invalid interpolation diagnostics
and orderly shutdown. The host log confirms it launches
`/home/idan/GitRepo/revo/zig-out/lsp/bin/revo lsp`, and Zed displays the new
interpolation diagnostic. The isolated host was stopped, clipboard text and
previous window focus restored, and installed binaries and personal editor
settings were preserved. No native GUI checks remained unavailable.

`git diff --check` passes in all six tooling repositories. The final independent
review has no unresolved substantive findings.

## Publication limits and editor recovery

The Zed extension cannot consume a new immutable grammar commit until that
commit exists in its public repository. The old public grammar pin remains
available; local parser/query checks exercise the pending changes. Updating
that public pin requires an authorized commit/push followed by the extension
update. The formatter adapters' automatic downloads likewise remain on their
existing published assets until an authorized new formatter release exists.
Configure the rebuilt local `revofmt` to use current validation now.

Compiler errors and editor recovery differ intentionally. Unknown interpolation
modes remain opaque `string` nodes for editing, and comment extras can bridge
range dots despite compiler literal-adjacency checks. The formatter rejects
both through its current upstream parser. Parse-only formatting can also
accept an ordinary range outside a loop that the compiler later rejects.
The grammar still has older compiler AST differences, including some ungrouped
arithmetic range endpoints; the refresh does not claim complete compiler AST
equivalence. Existing compiler-syntax fixtures include documented exclusions.

Native Linux x86_64 GNU is the tested formatter platform. Other OS/CPU targets,
package publishing and release installation were not tested or performed.
