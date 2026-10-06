# Revo formatter research

Historical research/review from 2026-10-05, covering the pinned revisions cited below. The formatter and static bridge have since been implemented and tested. Use the [current architecture](../docs/architecture.md) and [documentation index](../docs/README.md) for delivered interfaces, commands and later verification. Original observations and review evidence remain below.

Checked 2026-10-05 against upstream commit [`b571298`](https://github.com/if-not-nil/revo/tree/b571298b6fc95bc863548f118354c8d077792f6f).

I found no working formatter for Revo source code in the current upstream tools or the editor extensions checked. Formatting is explicitly pending in the bundled language server. This supports treating a Revo formatter as missing from the checked tools; it does not prove that no unpublished or unindexed project exists.

## Evidence

- The language server's feature list marks `textDocument/formatting` as `[TODO]`. The server's initialization advertises completion, navigation, rename, code actions, inlay hints and semantic tokens, but no formatting capability. There is no formatting handler in the server file. [LSP feature list](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lsp/README.md#L70-L92), [server capabilities and handlers](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lsp/server.zig#L102-L166).
- The formatting test exists but is skipped with `reason="TODO"`. It therefore does not demonstrate implemented formatting. [Formatting test](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/lsp/test.py#L926-L936).
- The CLI command table lists `compile`, `repl`, `lsp`, `dis`, `doc`, `version` and `bench`. It has no `fmt` or `format` command. [CLI commands](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/cli.zig#L410-L430).
- The documented VS Code extension starts the installed Revo binary with `lsp`; its source adds no separate formatter. The Zed extension also starts `revo lsp` and provides syntax highlighting. Neither supplies an independent formatting engine in the inspected source. [VS Code extension](https://github.com/PizzaLvr49/revo-lsp/blob/7a3c19f7d2fc982fa26179c5a2c5e1d073a7091c/src/extension.ts), [Zed extension](https://github.com/w0x7y/zed-revo/blob/34c1c796f90b2166b726596d6dcdd23d4fd80ef3/src/lib.rs), [editor setup documentation](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/docs/editors.md).

## Related names and search limits

Revo's `fmt(...)` function formats runtime values into strings. The repository's `zig build chore` runs Zig and Markdown formatters for maintaining the implementation and documentation. Neither reformats `.rv` source. [Runtime `fmt`](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/src/baselib/root.zig#L151-L156), [build maintenance step](https://github.com/if-not-nil/revo/blob/b571298b6fc95bc863548f118354c8d077792f6f/build.zig#L783-L804).

The search covered upstream source, documentation, all 92 issue and pull-request records and 19 discussion bodies returned by GitHub, plus web, GitHub repository and npm searches for Revo formatter, `revofmt`, `revo-fmt`, Prettier and Topiary projects. No applicable third-party formatter emerged. Discussion comments were not exhaustively inspected. The official website returned access errors, so its documentation was checked through the upstream repository. [Upstream repository](https://github.com/if-not-nil/revo), [issues](https://github.com/if-not-nil/revo/issues), [discussions](https://github.com/if-not-nil/revo/discussions).
