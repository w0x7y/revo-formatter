# Revo tooling architecture and Final Check, 2026-10-09

The architecture skill ran in all six tooling repositories, followed by
Final Check over the complete accumulated changes. The visual report and
candidate-selection steps were omitted at the user's request. Confirmed
findings were implemented and tested. Work remains local and uncommitted.

This record follows the
[upstream refresh](2026-10-09-upstream-refresh.md). That earlier record
describes its own stage; the tests and architecture below describe the final
working trees after this follow-up.

## Scope and pins

All checkouts are under `/home/idan/GitRepo/`. Review included unstaged changes
and new tests/documentation from each original HEAD, including the preceding
upstream refresh. Indexes remained empty. Existing changes and the original
two `revofmt-zed/graft/` JSON files were preserved. Their SHA-256 values still
match the snapshot taken before architecture work. Tool-created telemetry in
Neovim and VS Code was removed without deleting existing user files.

| Repository | Comparison base |
| --- | --- |
| revo-formatter | `f0cff5c2b0a82931ad57a2a4a7f4034d597f6d75` |
| tree-sitter-revo | `f15165b5391656ed3dcce25e18dbfba4320d80ed` |
| revo-zed-extension | `5385c5add959b1345265d3b24108c4b64aa90c66` |
| revofmt.nvim | `e3af3a00995ffe8008279d6e422892737425a6cd` |
| revofmt-vscode | `ad9ff9294886c13c82e1a9f252383699bfcf7dc0` |
| revofmt-zed | `6431ca7074defa54d7acfcaad96d3a63a91e1f0d` |

Upstream HEAD was checked again with:

```sh
git ls-remote https://github.com/if-not-nil/revo.git HEAD refs/heads/main
```

Both refs remain `e94e6d89ddaabb3249b38c1b10df87c700d1e8dc`, 38 commits
after `b571298b6fc95bc863548f118354c8d077792f6f`.

- Formatter frontend and Zed compiler audit pins changed from
  `b571298b6fc95bc863548f118354c8d077792f6f` to
  `e94e6d89ddaabb3249b38c1b10df87c700d1e8dc` during the refresh.
- The Zed public grammar pin remains
  `f15165b5391656ed3dcce25e18dbfba4320d80ed`. The updated grammar is
  uncommitted local source, not an invented replacement revision.
- Neovim's published formatter remains v0.1.1, source revision
  `1eb68eb07e5968cccc8b25ccad1357b8344b78c1`, SHA-256
  `bcc31238dff6b533c10a23c71e110d6e3211bbedb22dcde4d13f19b986e8937c`.
- The Zed formatter's server remains v0.2.0 with unchanged public URL,
  archive bytes and runtime checksums. VS Code's version remains unchanged.
- Cargo/npm dependency versions and lockfiles remain unchanged. The only
  Cargo manifest change in this phase excludes local Graft metadata from
  packaging. The optional TypeScript check uses a pinned external test tool.

## Implemented findings

**Formatter.** The oracle now owns input admission before parsing. Callers
receive an admitted `AnalyzedSource`; AST equivalence consumes analyzed pairs.
The public formatter borrows its initial input until admission, preserving the
requirement to reject excessive input before copying it. Trusted, shallow
collector stress tests explicitly bypass public admission inside the owning
module. `FormatOptions::validate` owns the ranges used by both library and CLI.
Invalid options still fail before help/version encountered later in the
argument list. The new direct-analysis regression failed before the fix and
passes afterward.

**Grammar.** The legacy numeric RANGE scanner token hid range punctuation from
expression parsing. For example, `for i in 0..2 + 3 do break i end` split its
iterator differently from a base-prefixed spelling. The scanner shortcut is
removed. The existing `range` node now has structured bounds, including
arithmetic/unary starts, steps and ends. Shared primary, binary and unary rules
retain ordinary unary precedence. Slices retain omitted bounds and whitespace
support. Numeric leaf-to-structured-node changes are documented and corpus
expectations were reviewed. Fifteen complete-program regressions execute
successfully with upstream Revo. Scanner whitespace and AST XML/recovery
handling each have one owner.

**Neovim.** The public installer reports completion exactly once, including
immediate failure to start. The command no longer repeats that reporting
policy. Real command failure/recovery and custom callback tests failed before
the fix and pass afterward.

**VS Code.** Failed, canceled and timed-out formatter requests close their
owned streams. An exited wrapper's descendant can otherwise retain inherited
pipes after the promise settles. Real subprocess/provider regressions failed
before the fix and pass afterward.

**Zed formatter.** The integrity module owns cache admission, removal, repair,
download revalidation and absolute entrypoint resolution through one interface.
Its launcher supplies the Zed download adapter. Caller tests cover warm cache,
relative paths with spaces, corruption, unexpected downloaded files, download
failure/retry, ordinary files and symlinks. Published server code and digests
remain unchanged.

**Final Check repairs.** Node binding metadata types now admit leaf entries and
model optional `children` as a descriptor object. A permanent typed caller
checks leaf, child, field and subtype access; it failed before the declaration
fix and passes afterward. VSIX and Cargo packaging now exclude Graft metadata.
Actual package inspection reproduced the leaks before the exclusions and
confirms their removal afterward.

The language extension needed no runtime architecture refactor. Its narrow
launcher and grammar-owned immutable parser lifecycle already concentrate
their policy. Its documentation now records 188 local corpus cases and the
published grammar's arithmetic-range limitation. Existing formatter
preservation/layout/renderer modules and editor codec/edit modules were
retained after the deletion test.

## Changed files and review coverage

The [refresh record](2026-10-09-upstream-refresh.md#changed-files) lists the
preceding pin/vendor/fixture changes. Additional or further edited files are:

| Repository | Architecture and final-review files |
| --- | --- |
| revo-formatter | `src/{lib,oracle,cli}.rs`, `src/tests/input_limits.rs`, `tests/cli.rs`, `CONTEXT.md`, `docs/{architecture,formatter,README}.md`, this record |
| tree-sitter-revo | `grammar.js`, `src/scanner.c`, generated `src/{parser.c,grammar.json,node-types.json}`, `test/ast/{revo_parser,check_expressions,check_generic_calls,check_upstream}.py`, `test/corpus/upstream-current.test` with reviewed numeric/recovery expectations and new cases, `bindings/node/{binding_test.js,index.d.ts}`, `test/types/node-metadata.mts`, `README.md` |
| revo-zed-extension | `docs/grammar-continuation.md`; the prior `tests/compatibility.toml` pin update remains |
| revofmt.nvim | `lua/revofmt/init.lua`, `doc/revofmt.txt`, `tests/install.lua`; prior README and formatting-test changes remain |
| revofmt-vscode | `src/transport.js`, `tests/{transport,provider}.test.js`, `tests/fixtures/formatter`, `docs/development.md`, `.vscodeignore`, `README.md` |
| revofmt-zed | `src/{lib,integrity}.rs`, `.gitignore`, `Cargo.toml`; prior README and LSP-test changes remain |

All changed items were reviewed. Generated parser output was checked through
the maintained grammar/scanner, deterministic regeneration, corpus, native
binding and Wasm builds. The 22 frontend source files and license match
upstream byte-for-byte; both vendor and historical-corpus checksum manifests
pass. Exhaustive AST comparison and generic collection continue to support
upstream's `global_const` without bridge or build dependency changes.

The repository-wide security review covered formatter bounded reads, admission,
FFI result ownership, AST/token preservation, atomic writes, build and package
inputs; grammar scanner/recovery/incremental behavior, C/Node/TypeScript
bindings, npm install/package configuration and Nix files; editor unsaved
buffers, codecs, stale/canceled requests, subprocess handles, deadlines, pinned
downloads, checksums, cache repair, LSP framing/settings and CI/package inputs.
Authentication/web/deployment paths do not exist in these local tooling repos.
No confirmed security issue remains within this inspected scope. Dependency
audits support that review and do not establish a general security guarantee.

## Commands and results

Commands run from each named repository. Shared current-tool settings:

```sh
export ZIG=/home/idan/GitRepo/revo-formatter/.tools/zig-x86_64-linux-0.17.0/zig
export REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt
export REVOFMT_CURRENT_SYNTAX=1
```

### revo-formatter

```sh
cargo test --lib analysis_admits_source_before_parsing -- --nocapture
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
cargo package --offline --allow-dirty
cargo audit
```

The focused test failed against the pre-refactor analysis entrypoint, then
passed. Final debug/release suites pass 100 tests each: 81 library, one binary
and 18 CLI. These include the 12,912 stress preservation/idempotence
combinations and fresh 2 MiB stack checks. Ten filtered Zig tests pass. Doc
tests run with zero examples. Clippy, formatting, build, package and checksum
checks pass. The compiler harness passes 20 programs at four widths, 80
compile/behavior/idempotence checks, 14 syntax rejections and one intentional
compiler-only rejection. Cargo audit reports no advisories for 13 locked
dependencies.

### tree-sitter-revo

```sh
tree-sitter generate
tree-sitter test --json-summary
python3 test/ast/check_generic_calls.py . queries/highlights.scm
python3 test/ast/check_expressions.py .
python3 test/ast/check_upstream.py .
npm ci
node-gyp rebuild
npm test
tree-sitter build --output /tmp/revo-final-parser.so
tree-sitter build --wasm --output /tmp/revo-final-parser.wasm
npm exec --yes --package=typescript@5.9.3 -- tsc --noEmit --strict --module nodenext --moduleResolution nodenext test/types/node-metadata.mts
npm audit --json
npm pack --dry-run --json
clang --analyze -std=c11 -I src src/scanner.c -o /tmp/revo-scanner-final.plist
```

All pass. Regeneration produces identical hashes for all three generated files.
Corpus: 188/188. AST checks: 18 positive/21 negative generic cases, 25 expression
cases, 48 valid whole programs, two intentional editor recoveries and seven
invalid loops including a missing body. Fresh native binding tests: 6/6,
including numeric-bound highlighting and incremental edits. Fifteen new
complete programs were executed with the exact e94 compiler. Additional
probes checked 379 incremental edits against fresh parses and bounded
operator/range/generic chains with a two-second timeout. Typed metadata
access passes, npm audit reports zero advisories, Clang analysis reports no
findings, and package dry-run admits 20 intended files while excluding tests,
graph caches, build output and dependencies.

### revofmt.nvim and revofmt-vscode

From each repo:

```sh
REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt REVOFMT_CURRENT_SYNTAX=1 scripts/verify
```

Neovim: 34 formatting and 15 installation tests pass on installed 0.12.5 and
the documented minimum 0.10.4. The latter executable is
`/tmp/revofmt-nvim010-final-HdLFvk/nvim-linux-x86_64/bin/nvim` and was selected
through a temporary PATH. Actual published v0.1.1 download/checksum/install,
unsaved formatting and idempotence also pass on the minimum host.

VS Code: 56/56 current-tool tests pass. Additional commands:

```sh
REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt npm run test:host
npm ci
npm run package
npm audit --json
```

Native VS Code 1.140.0 passes file recognition, activation, unsaved formatting
and idempotence. VSIX inspection confirms ten files, no graph cache and exact
runtime/manifest bytes. Audit reports zero advisories across 134 installed
development dependencies; the runtime extension has no npm dependencies.

Published-formatter compatibility was checked from both repos with
`REVOFMT_CURRENT_SYNTAX` unset:

```sh
REVOFMT_BIN=/tmp/revofmt-pinned-final-N7Ma99/revofmt scripts/verify
```

Neovim 0.10.4 passes 33+15 tests. VS Code passes 55 tests and skips the single
opt-in current-syntax rejection test as intended.

### revo-zed-extension

```sh
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo test --locked
cargo build --locked --release --target wasm32-wasip2
cargo audit
python3 tests/check_scopes.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_highlights.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_grammar.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_grammar_support.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_grammar_patch.py /tmp/revo-editor-audit-jl_c5279/grammar
python3 tests/check_compiler_syntax.py /tmp/revo-editor-audit-jl_c5279/grammar /home/idan/GitRepo/revo
```

All pass against the exact public f15165b grammar: 132 corpus cases,
18/21 generic AST cases, 25 expression cases, parser ownership/failure checks,
patch reproduction and 55 current compiler fixtures plus one documented
invalid fixture. The local parser also passes the actual Zed queries/scopes,
188 corpus cases and compiler contracts. Numeric children retain constant
captures and computed bounds retain variable captures. The launcher has no
unit tests; its build and actual host launch are the substantive checks.
Cargo audit reports no advisories for 88 locked dependencies.

### revofmt-zed and native Zed

```sh
REVOFMT_BIN=/home/idan/GitRepo/revo-formatter/target/release/revofmt REVOFMT_CURRENT_SYNTAX=1 scripts/verify
cargo build --locked --release --target wasm32-wasip1
cargo build --locked --release --target wasm32-wasip2
cargo audit
cargo package --list --allow-dirty --locked
scripts/package-server
python3 /tmp/revo-zed-final-smoke.py
python3 /tmp/revo-editor-audit-jl_c5279/check-revolt.py
```

All pass: 15 CLI/metadata checks, 79 framed Node tests, 13 Rust tests,
formatting, Clippy, four runtime digests and both Wasm builds. Cargo audit
reports no advisories for 97 dependencies. Cargo package listing excludes
the original Graft files. The server archive retains its five permitted
entries and published digest.

The isolated Zed test uses fresh WASI2 launchers, the public grammar and a
freshly downloaded public server. It verifies unsaved Unicode/literal and
interpolation formatting, idempotence, one undo, three syntax rejections and
unchanged disk files. Current Revolt passes initialize, hover, definition,
rename, completion, invalid-interpolation diagnostics and orderly shutdown.
Evidence is under `/tmp/revo-zed-final-vuhdm8fx`. Only temporary profile/trust
and native test harness setup was adjusted; personal settings were unchanged.
Clipboard and prior window focus were restored and focus restoration asserted.

## Remaining limits

No required local check remains blocked and no confirmed finding remains
unresolved. `git diff --check`, `git diff --cached --check` and new-text
whitespace checks pass in all six repos; changed Markdown relative links were
checked. All indexes remain empty.

`revo-zed-extension` has no runtime architecture changes; its prior audit pin
and updated compatibility documentation remain. No tooling repo is entirely
unchanged across both phases. The upstream Revo checkout has no source changes.

The grammar pin cannot move to an unpublished local revision. Automatic
formatter downloads likewise retain available release assets. Updating these
published pins requires an authorized publication step. Local editor checks
use the rebuilt formatter and local grammar checks use the updated parser.
Opaque interpolation and comment-extra recovery intentionally differ from
compiler errors; the formatter uses current compiler parsing and rejects
invalid input without emitting edits.

Native Windows/macOS, remote editor workspaces, VS Code 1.85, Bun and Nix-shell
execution were not tested. The formatter remains verified on Linux x86_64
GNU. npm blocked an optional VSIX signing postinstall under its existing
policy; unsigned packaging passed and signing was outside the requested scope.
The preceding feature-disabled upstream test run's regex-documentation failure
remains documented in the refresh record; the regex-enabled upstream suite
passed. No publishing, pushes, PRs, releases or installed-binary replacement
was performed.
