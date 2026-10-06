# Final whole-branch review

Historical review of the original editor-integration stage on 2026-10-06.
Line references, readiness and host limitations below apply to the reviewed
range. Later activation, launcher and timeout fixes have their own records;
see the [documentation index](../README.md) and
[later source/editor check](2026-10-06-editor-final-check.md). The
[editor guide](../../editors/README.md) describes current packages.

Reviewed range: `e048b7d5..520f2374`, 2026-10-06.

### Strengths

- The package boundaries are sensible and self-contained. Neovim separates raw transport, byte representation, and buffer lifecycle; VS Code separates transport, edit calculation, settings, and provider wiring. Zed stays declarative. The shared runner selects one absolute executable and invokes the actual package runners (`scripts/verify-editors:4`). Cargo exclusions keep the editor tooling out of the Rust source package (`Cargo.toml:8`). No formatter, bridge, vendor, pin, or existing Rust test changes appear in the reviewed range.
- Neovim checks buffer generation, changedtick, loaded/modifiable state, and serialization options before applying output (`editors/neovim/lua/revofmt/init.lua:83`). Its raw transport rejects nonzero exits, signal termination, excessive output and timeout; its codec requires an exact round trip rather than stripping carriage returns globally (`editors/neovim/lua/revofmt/transport.lua:24`; `editors/neovim/lua/revofmt/codec.lua:24`). The synchronous save hook is opt-in and setup is repeatable.
- VS Code rejects lossy UTF-16 encoding, invalid UTF-8 output, nonzero/signal exits, cancellation, and output overflow (`editors/vscode/src/transport.js:7`). Provider trust, version, EOL, generation and disposal guards protect newer edits (`editors/vscode/src/provider.js:19`). The contiguous replacement avoids splitting surrogate pairs or CRLF and checks reconstruction under editor normalization (`editors/vscode/src/edits.js:9`). The position round trip provides a second representation guard.
- Both procedural adapters use executable/argv invocation and whole unsaved-buffer stdin. Defaults and bounds agree with the CLI. The VS Code extension has no runtime npm dependencies; all 134 locked dependencies are development-only registry packages. The existing VSIX has the expected ten entries, and its five runtime sources and license match the reviewed checkout byte-for-byte.
- The Zed snippet explicitly disables both native whitespace passes, as required to protect opaque source and failed formatting (`editors/zed/settings.json:11`). Its guide clearly separates native-host limitations from direct CLI evidence and describes the existing-language installation path (`editors/zed/README.md:21`, `:97`, `:141`).
- Tests exercise real CLI preservation/idempotence through its public process interface, plus focused transport and lifecycle fixtures. The logs support the reported 27 Neovim, 45 VS Code, and 15 Zed checks. The recorded full Rust/Zig verification is consistent with the supplied logs. Documentation does not present provider doubles or command tests as native editor-host tests.

### Issues

#### Critical (Must Fix)

None found.

#### Important (Should Fix)

None found.

#### Minor (Nice to Have)

None found.

### Recommendations

- Retain the documented host verification limits. When suitable isolated hosts are available, execute the existing VS Code and Zed manual smoke procedures and a Neovim 0.10 run. These are follow-up evidence improvements, not findings against this approved locally installable scope.
- Complete the final-review bookkeeping in the implementation plan and verification record using this report. Their current pending status is accurate for the reviewed commit, which necessarily preceded this review.

### Decision and deferred-item assessment

No parked actionable review finding was present in the task reports, reviews, or progress ledger. The Neovim signal acceptance defect was fixed and independently re-reviewed before this final range.

The Neovim native undo/EOL decision is proportionate: one text undo region and explicit disclosure avoid introducing global undo machinery. Empty-buffer ambiguity, BOM/non-UTF-8 rejection and representability refusal are disclosed restrictions rather than silent byte changes (`editors/neovim/README.md:77`).

The Zed LF restriction is an intentional, material scope limitation, prominently included in the approved spec and package guide. Independent inspection confirms that its native external formatter passes rope text, checks process success and UTF-8, and applies through an undo-transaction guard; its whitespace passes precede the command. That supports retaining the native integration with both passes disabled. It does not establish host execution or expand the guarantee to CRLF/mixed files. [Zed native formatting source](https://github.com/zed-industries/zed/blob/main/crates/project/src/lsp_store.rs)

Zed's diff pipeline normalizes new text and records its detected line ending, supporting the documented limitation. [Zed buffer source](https://github.com/zed-industries/zed/blob/main/crates/language/src/buffer.rs)

The grammar-free package is supported by the current optional grammar field and extension-directory discovery; this remains current-source evidence, not proof for every historical release. [Language configuration](https://github.com/zed-industries/zed/blob/main/crates/language_core/src/language_config.rs), [extension builder](https://github.com/zed-industries/zed/blob/main/crates/extension/src/extension_builder.rs)

VS Code's current native text buffer normalizes inserted EOL sequences to the document setting, supporting the adapter's exact reconstruction check. [VS Code text-buffer implementation](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts)

### Review evidence and limits

Read the prepared whole-branch review package, approved spec and plan, contributor instructions, current architecture/domain/build guides, implementation files, tests, install documentation, package metadata, task reports/reviews and decisions. Inspected tracked file modes, packaging exclusions, lockfile metadata, the existing VSIX, and verification logs. Commit history and task records show Neovim, its signal follow-up, VS Code, then Zed, with independent reviews between editor implementations.

No already-passing suite was rerun. No concrete uncovered behavior warranted a new execution probe. The read-only packaging inspection and primary-source checks above add narrower evidence; they do not establish native VS Code/Zed host behavior. No tracked files, index, HEAD, branch, or personal editor settings were changed. This ignored review report is the only written file. No subagents were dispatched.

### Assessment

**Ready to merge? Yes.**

**Reasoning:** The branch implements the approved editor scope with compact ownership, guarded process/edit paths, correct packaging, and substantial verification evidence. No actionable defect was found; native host testing and the documented editor representation limits remain explicit boundaries on the evidence and guarantees.
