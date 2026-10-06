# Editor integrations verification — 2026-10-06

## Scope and structure

The implementation starts at `e048b7d5` and adds locally installable packages
under `editors/neovim/`, `editors/vscode/`, and `editors/zed/`, in that order.
Each owns its metadata, source, tests, installation guide and full MIT license.
The [shared guide](../../editors/README.md) describes the CLI contract and
`scripts/verify-editors` invokes all three package runners.

No formatter source, bridge, existing Rust tests, upstream pin or vendor files
changed. Editor files and development dependencies are excluded from the Cargo
source package. No registry publication, remote push or personal editor
configuration change was performed.

## Verification evidence

| Check | Result |
| --- | --- |
| Neovim headless package tests | 27 passed, zero failures |
| VS Code process/provider tests | 45 passed, zero failures |
| Zed metadata and actual CLI tests | 15 passed, zero failures |
| `cargo test --all-targets` | 77 passed: 60 library, one binary, 16 CLI |
| `cargo test --doc` | Passed; zero doctests |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo fmt --check` and release build | Passed |
| Exact Zig 0.17.0 formatting and local filtered tests | Passed; nine tests |
| Vendor and corpus checksum manifests | Passed; 23 and 27 files respectively |
| `bash -n scripts/verify-editors` and diff whitespace check | Passed |
| Guide links and package licenses | Local links resolved; all licenses byte-identical to root |
| Cargo package file list | Editor packages, dependencies and scratch files excluded |

The shared runner passed from `/tmp`, using its default absolute release binary.
Package-specific checks also ran with explicit `REVOFMT_BIN` overrides. The
successful real CLI cases exercise its preservation and idempotence interface;
editor tests do not import private formatter implementations.

Verification used native Linux x86_64 GNU, Neovim 0.12.5, Node.js 26.10.0,
npm 12.2.0 and Python 3.14. Logs were recorded locally under
`target/editor-verification/`; generated artifacts are ignored.

VS Code packaging used the pinned development-only `@vscode/vsce` 4.0.0.
`npm ci` reported zero vulnerabilities. The locally generated
`editors/vscode/revofmt-0.1.0.vsix` contains ten entries; its identifier,
runtime sources, README and license were inspected against the package sources.
Its SHA-256 is
`5bb9ea450837c84efca8fb00486e00e4c59f09b82e99a238d245dad2c74b80a3`.

## Independent reviews

Fresh reviewers checked specification compliance and code quality after each
implementation, before the next editor implementation began.

| Package | Implementation | Review |
| --- | --- | --- |
| Neovim | `0444dc12` | Approved; no findings |
| Neovim signal termination follow-up | `9338fac7` | Scoped re-review approved; no findings |
| VS Code | `b7409663` | Approved; no findings |
| Zed | `4be49fbc` | Approved; no findings |

The Neovim follow-up came from a focused coordinator probe: a signal-terminated
`vim.system` child can return exit code zero with a nonzero signal. Two failing
tests reproduced the unsafe acceptance before the transport began rejecting
signals in both synchronous and asynchronous modes.

The final whole-branch review is pending at this recorded stage.

## Decisions and limits

- The user's approved design authorized implementation without another design
  approval gate. Documentation refinements are reversible if that interpretation
  needs correction.
- All root README verification commands ran for these editor changes, following
  the contributor contract; the cost was verification time.
- Neovim uses one native text undo region. Native undo does not restore the
  `endofline` option, so an EOL-only change may need an option reset separately.
  Adding custom global undo machinery would broaden the plugin considerably.
- Zed's native buffer pipeline normalizes line endings. The supported
  preservation scope is UTF-8 LF source; CRLF and mixed-ending files may lose
  opaque bytes. Broader preservation needs a different integration architecture.
- Zed settings disable native trailing-whitespace removal and final-newline
  insertion because those passes run before the external command. Users must
  retain those settings to protect opaque bytes and failed-format buffers.

Actual VS Code extension-host testing was unavailable. Zed 1.22.0 was present,
but its native host was not exercised. Package tests and current primary-source
inspection establish narrower evidence than running either editor: activation,
native undo, cancellation and stale-result behavior remain host smoke-test
items. Source inspection does not establish a minimum compatible Zed release.
Each package guide provides installation and manual smoke-test instructions.
