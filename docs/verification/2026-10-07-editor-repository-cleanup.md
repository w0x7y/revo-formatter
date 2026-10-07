# Formatter repository cleanup

This records the cleanup after Neovim, VS Code and Zed moved to dedicated public
repositories. The comparison base is `a480fc40bb1fbbcf4379f509f3dad41837c90988`.
Current instructions live in the [documentation index](../README.md).

## Scope

Removed the duplicate `editors/` packages and `scripts/verify-editors`. Their
source, licenses, tests and guides remain in the dedicated repositories and
in Git history. Before removal, the copied VS Code runtime, Neovim transport and
codec, and Zed settings and metadata were compared with the standalone packages.
The VS Code and Zed repositories were also verified from public fresh clones
during extraction.

Removed editor-only Cargo exclusions and ignore entries. Updated the project
README, contributor instructions, glossary, architecture, reference and
third-party build guidance. The [integration guide](../editors.md) now records
the public CLI contract and points to downstream setup and checks.

Historical editor reports keep their recorded results and paths. Links to the
former shared guide point to the immutable pre-extraction source snapshot.
The previous documentation handoff is indexed as historical evidence.

The Rust and Zig runtime, dependency lockfile, frontend revision, vendor source,
corpus inputs, reviewed output fixtures and license notices are unchanged.
No personal editor settings are changed.

## Verification

Checks ran on native Linux x86_64 GNU with exact Zig 0.17.0. Rust commands
used cached dependencies with `--offline --locked`.

| Check | Result |
| --- | --- |
| `cargo test --all-targets` | 72 library, 1 CLI unit and 16 CLI process tests passed |
| `cargo test --release --all-targets` | The same 89 tests passed, including 2 MiB stack admission regressions |
| `cargo test --doc` | Passed; there are no doctests |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo fmt --check` and Zig format checks | Passed |
| Filtered local Zig `ReleaseSafe` tests from the README | All 10 passed |
| `cargo build --release` | Passed |
| Vendor and corpus checksum manifests, run in their directories | All entries passed |
| `cargo audit` | No advisories reported for the 13 locked dependencies |
| `cargo package --offline --locked --allow-dirty` | Packaged and verified the extracted source package |
| Markdown links and heading anchors | All 216 local links passed |
| README examples and whitespace | Two-space indentation retained; no whitespace errors |

The source archive contains 124 files. It includes the library, CLI, Zig bridge,
pinned frontend, corpus fixtures, provenance, licenses and updated guides. It
contains no editor packages, editor verification script, VSIX files, npm
dependencies, local toolchains or build caches.

The three documented downstream commands ran against the rebuilt release CLI:

- Neovim: 32 adapter and 13 installer tests passed.
- VS Code: all 49 unit and process tests passed.
- Zed: all 15 metadata and CLI tests passed.

Native VS Code and Zed host checks were completed during extraction; they were
not repeated for this removal of duplicate files and documentation update.
Zed's checks here exercise metadata and the CLI, not native edit application.
The standalone repositories retain their host-specific coverage and limits.

## Review coverage

Reviewed all removed package paths against the standalone owners, the obsolete
runner, Cargo exclusions, ignore entries, current guide changes and historical
link repairs. Formatter source, dependency pins and attributed inputs remain
byte-identical to the comparison base. The Git index was unchanged during review.

The security review inspected CLI stdin and path handling, atomic replacement,
input admission, the FFI response boundary and build-tool invocation. Existing
public-boundary regressions and the dependency audit passed; no actionable
security finding was identified in that inspected scope. This is a bounded
review, not a guarantee of exhaustive vulnerability coverage.
