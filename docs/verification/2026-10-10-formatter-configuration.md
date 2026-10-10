# Formatter configuration verification, 2026-10-10

Tab indentation, a configurable blank-line limit and `revofmt.toml` discovery
were added to the formatter and its CLI, and the crate version moved from 0.1.2
to 0.2.0. This record covers the formatter repository and the current editor
adapters' own checks run against the rebuilt candidate. The editor adapters
have not been updated and v0.2.0 is not published.

## Scope and commit range

The [approved design](../superpowers/specs/2026-10-10-formatter-configuration-design.md)
and [implementation plan](../superpowers/plans/2026-10-10-formatter-configuration.md)
were executed in a single worktree. The code base is `f9c95329` ("Pin the Revo
frontend to upstream f0034ab"). Verification ran on `7ef989f5`, with a clean
working tree before this record was updated. An earlier pass on `2edf0d4a`
preceded the final whole-branch review, whose fixes are the last three rows.

| Commit | Change |
| --- | --- |
| `dcb05d69`, `e173d728` | Design and plan |
| `901958b6` | `IndentStyle` and `max_blank_lines` in `FormatOptions` |
| `04947eb7` | Tab indentation rendering |
| `f0815129` | Configurable blank-line limit |
| `f1b44852`, `50e42b64` | Corpus and stress checks at every option set; provenance counts |
| `730c54b9` | `--indent-style` and `--max-blank-lines` flags |
| `15252f96` | `revofmt.toml` discovery and parsing |
| `2edf0d4a` | Per-input option resolution |
| `aada4704`, `fbc92755` | The first version of this record and its index entry |
| `fbdf10ae` | Discovery starts at the real location of the deepest existing ancestor |
| `c68ed258` | Configuration reference, help footer, architecture date and a test comment |
| `7ef989f5` | Tests for nested key paths, symlinked files and the adapter argv |

`f9c95329..7ef989f5` changes 22 files. The new dependency is `toml` 1.1.6 with
only the `parse`, `serde` and `std` features. Its closure adds `equivalent`,
`hashbrown`, `indexmap`, `serde_spanned`, `toml_datetime`, `toml_parser`,
`toml_writer` and `winnow` to `Cargo.lock`. The vendored Revo frontend, its
checksums and the upstream pin are unchanged. The Zig bridge is unchanged.

## Environment

```sh
export ZIG=/home/idan/GitRepo/revo-formatter/.tools/zig-x86_64-linux-0.17.0/zig
```

`"$ZIG" version` prints `0.17.0`. The platform is native Linux x86_64 GNU.
`cargo package` and the baseline build below used `--offline`.

## Commands and results

Commands ran from the repository root and all exited 0.

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

| Check | Result |
| --- | --- |
| Debug tests | 140 pass: 88 library, 21 binary (`config`, `cli`) and 31 CLI process tests |
| Release tests | The same 140 pass |
| Doc tests | Zero examples, zero failures |
| Clippy, `cargo fmt --check`, `zig fmt --check` | No findings |
| Zig tests | 10 of 10 filtered tests pass |
| Release build | Succeeds |
| `vendor/revo` checksums | 23 files OK |
| `tests/fixtures/upstream` checksums | 27 files OK |
| `cargo package --offline --allow-dirty` | Packaged and verified `revofmt` 0.2.0: 138 files, 1.5 MiB (367.3 KiB compressed) |

The previous full-suite record
([architecture and Final Check](2026-10-09-architecture-final-check.md)) had
100 tests per profile: 81 library, one binary and 18 CLI. The additions are
formatting cases for tabs and blank-line limits, the configuration module's
unit tests (including nesting on 2 MiB threads, the size limit and discovery
through missing directories, `..` and symbolic links) and the CLI process tests
for the new flags, discovery, precedence and the adapter argv.

### Corpus and stress combinations

The corpus covers widths 24, 80 and 120, indent widths 2 and 4, both indent
styles and blank-line limits 0, 1 and 2: 36 option sets, 720 input/option
combinations. The four reviewed outputs apply to space indentation and limit 1.
The stress fixtures run 77,472 preservation and idempotence combinations
(288 file-read match, 288 complete file, 14,688 across 51 sections and
62,208 composed), up from 12,912.

The corpus module was re-run on its own, twice per profile, because the larger
matrix lengthened it:

```sh
cargo test --lib tests::corpus
cargo test --release --lib tests::corpus
```

| Profile | Tests | Durations | Starting commit |
| --- | --- | --- | --- |
| Debug | 8 | 15.99 s, 16.16 s | 2.64 s |
| Release | 8 | 4.60 s, 4.55 s | 0.77 s |

The roughly sixfold increase matches the sixfold growth in cases. Within the
full debug and release runs, the library test binary finished in 15.85 s and
4.51 s.

## Default-output comparison

Default options must produce the bytes the starting commit produced. The
comparison was repeated on `7ef989f5`. A detached worktree of `f9c95329` was
created at `/tmp/revofmt-base` and built with `cargo build --release --offline`.
It was removed with
`git worktree remove /tmp/revofmt-base` (no `--force`), and `git worktree list`
afterwards shows only the repository's own worktrees.

Both binaries ran from `/tmp`. No `revofmt.toml` exists in `/tmp`, in `/` or in
any ancestor of the repository path. Each fixture was formatted with no flags,
once by the old binary and once by the new, using the file as an argument and
again on stdin. The new binary also ran with `--no-config`, which the old
binary does not accept.

| Group | Files |
| --- | --- |
| `tests/fixtures/upstream/*.rv` | 20 |
| `tests/fixtures/upstream/expected/*.rv` | 4 |
| `tests/fixtures/upstream/rejected/*.rv` | 1 |
| `tests/fixtures/stress/*.rv` | 3 |

All 28 files are identical: exit code and stdout bytes match across the old
binary, the new binary and the new binary with `--no-config`, in file and stdin
modes. The rejected fixture exits 2 with empty stdout in all of them. No default
output differs from `f9c95329`.

## Final-review fixes

The final whole-branch review found one behavior defect and some inaccurate
documentation and test coverage.

- **Discovery walked non-ancestors.** When the input's parent directory could
  not be canonicalized, discovery walked the raw joined path. From a project
  with a `revofmt.toml`, `--stdin-filepath ../other/new/x.rv` (with `new/`
  missing), a path through a symbolic link that points out of the project, and
  `project/../outside/new/x.rv` all used the project's configuration instead
  of the configuration at the real location. Discovery now starts at the
  canonical form of the deepest existing ancestor of the parent directory.
  The new `..` and outward-symlink tests failed on the previous discovery code
  (they received the project's `indent_style = "tab"`) and pass now. A missing
  subdirectory of a configured project still finds that project's file, and a
  file used as a directory falls back to its ancestors.
- **Reference accuracy.** `docs/formatter.md` now says a symbolic link to a file
  uses the configuration found from the directory holding the link, that a file
  over 64 KiB is an error, and that ranges are checked on resolved options, so a
  flag can override an out-of-range key. The help footer says "nearest ancestor
  directory".
- **Test coverage.** Dotted keys, table headers, arrays of tables, inline dotted
  keys and a nested dotted key at depth 30,000 are errors on a 2 MiB thread
  and carry the parser's recursion limit message, matched as `recurs` so either
  wording passes. A CLI process test runs the exact adapter argv with and
  without a project file: the file wins and its omitted keys use built-in
  defaults, and without a file the flags apply.

## Editor adapter checks

The current adapters' documented checks ran against the rebuilt release
binary (`target/release/revofmt`, `revofmt 0.2.0`) through
`REVOFMT_BIN="$PWD/target/release/revofmt" <adapter>/scripts/verify` from the
formatter root. The Zed Rust checks ran with `CARGO_NET_OFFLINE=true`. No
packages were installed, and `git status --short --ignored` in each adapter
repository was identical before and after.

| Adapter | Commit | Result |
| --- | --- | --- |
| `revofmt.nvim` | `fa1ac47` | CLI tests: 33 pass, 0 failures. Installer tests: 8 of 15 pass, 7 fail |
| `revofmt-vscode` | `2c87ee9` | 55 pass, 0 fail, 1 skipped; 56 pass with `REVOFMT_CURRENT_SYNTAX=1` |
| `revofmt-zed` | `8868a8b` | Python 15 pass; Node 78 pass, 1 skipped (79 pass with `REVOFMT_CURRENT_SYNTAX=1`); Rust 13 pass; `cargo fmt --check`, clippy and `scripts/package-server --check` pass |

The Neovim installer failures are caused by the version pin, not by formatting
behavior. `lua/revofmt/release.lua` pins the managed release to 0.1.2, and its
installer tests publish the supplied binary only if `--version` begins with
`revofmt 0.1.2 `. The candidate prints `revofmt 0.2.0`, so each install fails
with "formatter cannot run on this system" and an empty detail. The same
`scripts/verify` against an existing 0.1.2 build passes all 15 installer tests.
They are expected to pass once the adapter pins release 0.2.0. With
`REVOFMT_CURRENT_SYNTAX=1` the Neovim CLI tests also pass (34, with the
compiler syntax rejection case) and the same seven installer tests fail.

## Limits

- The editor adapters (Neovim, VS Code and Zed) have not been updated. They do
  not yet pass `--stdin-filepath` or honor `revofmt.toml`, and they still use
  the published pre-0.2.0 formatter. Their current checks pass against the
  candidate except the Neovim installer tests, which fail on the 0.1.2 version
  pin as described above.
- No native editor session was run for this stage.
- v0.2.0 is not tagged, pushed or published, so there is no published binary
  asset or checksum. Adapter work depends on that publication.
- `scripts/check-upstream` (compiler acceptance and execution) and `cargo audit`
  were not run for this stage. The compiler harness checks spaces at four
  widths, so tab-indented output and non-default blank-line limits are verified
  by token-tape and syntax-tree preservation and idempotence, not by executing
  programs.
- Tabs inside opaque literals and comments are still not measured, as before.
- Verification covers native Linux x86_64 GNU with Zig 0.17.0 only.
