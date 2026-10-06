# Documentation and merge preparation, 2026-10-06

This records the documentation refresh and local package checks after the
[editor final check](2026-10-06-editor-final-check.md). The feature begins at
`main` merge base `e048b7d598608aa702bf38713334fb87bb5ae159`; the review includes
all editor integrations, source-block ownership changes, and the admission and
Neovim deadline repairs. Runtime sources are unchanged from the final-check
verification. The [documentation index](../README.md) identifies current guides.

## Documentation coverage

All 43 maintained Markdown documents were inspected. Current root and editor
guides describe the implemented interfaces, installation, resource admission,
buffer preservation, verification commands and known platform limits.
`AGENTS.md` now covers editor ownership, source-block provenance, mixed-form
admission tests and documentation/package verification. The domain glossary
defines those boundaries and the editor's buffer representation.

The 29 plans, designs, research notes and older verification/review records
identify their recorded stages and link to current guides. Their original test
counts, pins, hashes, commit IDs, benchmarks, witnesses and verdicts remain
historical evidence. The earlier editor final check has its own stage notice.
The index includes the activation/editor reviews and labels completed plans.

Corpus provenance now points to the verification history, describes checks of
each corpus result, and records the pipe-expression newline limitation.
Its checksum manifest was regenerated from the existing file list; upstream
fixture bytes, goldens, vendor source and license notices remain unchanged.
Generated Graft notes, build output, dependencies and immutable upstream files
are outside the maintained-documentation scope.

## Verification

All executable checks use native Linux x86_64 GNU and exact Zig 0.17.0. The
unchanged runtime sources retain the preceding final-check evidence: 89 Rust
tests in each of debug and release, ten local Zig tests, doctests, clippy, Rust
and Zig formatting, release build, 32 Neovim/49 VS Code/15 Zed tests and native
VS Code 1.140.0 coverage. Its repository security review and zero-advisory
Cargo/npm audits remain stage-specific evidence, not a security guarantee.

| Refresh check | Result |
| --- | --- |
| Complete Markdown inventory | 43 documents, 221 local and two canonical repository targets, 14 Markdown anchors; zero errors |
| Fences, whitespace and examples | Passed; three JSON examples parsed and four Lua snippets compiled without executing user configuration |
| Runtime source continuity | All six modified source/test files match the final-check snapshots |
| Vendor and corpus manifests | Passed from their directories; 23/27 entries; only the provenance hash changed |
| Focused corpus test | Passed all 120 input/option combinations with preservation and idempotence |
| Rebuilt VSIX 0.1.1 | Ten entries; runtime, manifest, README and license match source bytes; development files excluded |
| Offline Cargo package and extracted build | Passed; 121 files; editor/dependency/scratch exclusions retained |

The package check used `cargo package --offline --allow-dirty` with exact `ZIG`;
the allowance covered the source repairs and documentation awaiting their
commit. Subsequent completion edits only record this verification evidence.

The VSIX remains version 0.1.1 and has SHA-256
`985303c55f56e8ff9795bbfe251ffa4f21d6cd39f577c3a920aedd109f407d20`.
Packaging exposed incorrect rewriting of the guide's relative repository links;
explicit repository URLs fixed it and were independently re-reviewed. The
rebuilt archive's complete guide now matches the checkout. It remains an ignored
local artifact; no registry publication is part of this handoff.

## Reviews and limits

Fresh independent reviewers approved the editor guides, their packaging-link
correction and all 29 historical documents. The root-guide review found no
actionable defect and checked the final evidence-record completion separately.

Minimum Neovim 0.10 and VS Code 1.85 remain unexecuted here. Native VS Code
coverage establishes recognition, automatic activation, unsaved buffers, edits
and idempotence on 1.140.0; remote hosts, Restricted Mode and native
undo/cancellation/stale-result/EOL/save scenarios remain unverified. Zed checks
are metadata and actual CLI execution, not a new native-host run, and its
preservation scope remains LF. Neovim deadlines depend on the editor loop and
terminate the direct formatter child; wrappers own their descendants. The
2 MiB stack probes are bounded regression evidence for the pinned configuration.

The primary checkout is the durable installation location after merging.
Local editor paths that referred to the deleted worktree must point to a
persistent checkout or installed formatter. The editor guides use generic
paths, and this task does not edit personal editor configuration.

Local review reports and command evidence are retained in the primary checkout's
ignored `target/documentation-handoff/` directory when the temporary worktree
is removed. Source and documentation checks complete before the commit, push
and merge; the Git outcome is reported separately after those actions succeed.
