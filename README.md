# revofmt

`revofmt` is a Rust CLI and library for formatting Revo source. It validates
every result with the pinned upstream frontend and preserves syntax modulo
source coordinates. The supported syntax is that of Revo revision
`b571298b6fc95bc863548f118354c8d077792f6f`.

The initial supported and verified build platform is native
`x86_64-unknown-linux-gnu`. The build rejects other host/target combinations.
Other platforms need separate build and ABI validation.

Locally installable integrations provide formatting and file recognition for
[Neovim](editors/neovim/README.md), [VS Code](editors/vscode/README.md), and
[Zed](editors/zed/README.md). They format unsaved buffers through an installed
`revofmt`; save formatting is opt-in. Start with the
[editor integration guide](editors/README.md).

## Repository layout

| Directory | Contents |
| --- | --- |
| `src/` | Rust library, CLI, layout, and preservation tests |
| `bridge/` | Zig interface to the pinned frontend |
| `vendor/revo/` | Unchanged upstream source and checksums |
| `tests/` | CLI process tests and attributed corpus fixtures |
| `editors/` | Self-contained Neovim, VS Code, and Zed packages |
| `scripts/` | Repository verification commands |
| `docs/` | Current guides, designs, plans, and verification records |
| `research/` | Dated upstream investigations |

## Build from source

Install Rust with edition 2024 support and exact stable Zig **0.17.0**.
The build script uses `zig` from `PATH`, or the executable selected by `ZIG`.
It never downloads tools or source. See [THIRD_PARTY.md](THIRD_PARTY.md) for
the pinned archive checksum, vendored source provenance, and license notices.

With Zig 0.17.0 on `PATH`:

```sh
zig version
cargo build --release
cargo test --all-targets
```

Or select its executable explicitly:

```sh
ZIG=/absolute/path/to/zig-0.17.0/zig cargo build --release
ZIG=/absolute/path/to/zig-0.17.0/zig cargo test --all-targets
ZIG=/absolute/path/to/zig-0.17.0/zig cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The resulting `target/release/revofmt` statically includes the Revo frontend.
Zig is a compile-time dependency. Running the binary requires no installed
Zig or Revo executable; normal Linux system libraries still apply.

## CLI usage

```sh
# Read stdin and print formatted source. An explicit - also reads stdin.
printf 'let x=1' | target/release/revofmt
printf 'let x=1' | target/release/revofmt -

# Print one file without editing it.
target/release/revofmt example.rv

# Check or write several explicit files.
target/release/revofmt --check first.rv second.rv
target/release/revofmt --write first.rv second.rv

# Override layout settings.
target/release/revofmt --indent-width 4 --line-width 24 example.rv

# A filename starting with a dash follows --.
target/release/revofmt -- --example.rv

target/release/revofmt --help
target/release/revofmt --version
```

For example, `let x=1` becomes `let x = 1` followed by a newline.
Width-driven formatting, tested with `--line-width 24`, changes:

```revo
print(first_argument, second_argument)
```

into:

```revo
print(
  first_argument,
  second_argument
)
```

Defaults are two spaces per indentation level and 80 display columns.
`--indent-width` accepts 1 through 8; `--line-width` accepts 20 through 240.
Width is a soft target. Long literals, comments, and syntax that cannot safely
break can exceed it. The formatter can retain compact spacing through its
validated conservative fallback. Existing statement newlines and comment
boundaries are retained. Short expression continuations, argument lists and
return tables collapse onto one line when they fit. Unary minus stays attached
to its operand, and indexing uses `nums[y]`.

Control-flow headers are measured independently of their `do ... end` bodies.
Generic lists have their own formatting group; expanded lists put each argument
on its own indented line. Long call arguments can wrap while a short generic list
stays inline. Operator chains, including comparisons and pipes,
fill available columns with a single continuation indentation level. Trailing
operator comments stay attached to the preceding line. Function expression
bodies without `do` stay inline when short and receive indentation when expanded.
Parser-sensitive whitespace can still require conservative layout, and long
unbreakable expressions or comments can exceed the target.

Print mode accepts one input. `--check` also accepts stdin, including when
no input is specified. `--write` requires file paths and rejects stdin,
symlinks, and nonregular files. `--check` and `--write` are mutually exclusive.
There is no directory discovery or configuration file support in this version.

The CLI and library apply conservative input limits before parsing: 262,144
UTF-8 source bytes, 4,096 expanded lexer tokens, and 32 combined delimiter and
embedded-source levels. Shared weighted budgets also limit recursive parser
forms, AST traversal, and layout, including flat operators and postfix chains.
Interpolation and quasiquote bodies count toward these budgets even though their
literal bytes remain opaque to formatting. Ordinary literal and comment contents
count toward the byte limit, without counting their punctuation as syntax.
Limits apply to generated candidates too; space for a final newline can therefore
be necessary. A limit failure returns `FormatError::Validation`, or CLI exit code
2 with no output or batch writes. CLI reads stop after one byte beyond the byte
limit. See [the exact admission policy](docs/verification/input-limits.md).

Formatted source goes to stdout only in print mode. Check differences and
errors go to stderr with their input names. Exit codes are:

| Code | Meaning |
| --- | --- |
| 0 | Success, or all checked inputs already formatted |
| 1 | At least one checked input needs formatting |
| 2 | Usage, I/O, syntax, or validation error |

An error in any checked input takes precedence over code 1. Invalid syntax
produces no formatted source. Write mode computes and validates every input
before changing any file, so a malformed or unreadable input prevents all
writes. Each changed file uses a new temporary file in the same directory,
preserves the original permissions, flushes its contents, and replaces the
destination atomically. Unchanged files keep their existing inode and
modification time. Temporary files are removed on errors, with a diagnostic
if cleanup itself fails.

Multiple file writes are not a single transaction. A later replacement or
other I/O failure can leave earlier writes completed. The diagnostic lists
those completed paths so the result can be inspected.

## Rust library usage

Use this checkout as a local dependency:

```toml
[dependencies]
revofmt = { path = "/absolute/path/to/revo-formatter" }
```

```rust
use revofmt::{FormatError, FormatOptions, format};

fn main() -> Result<(), FormatError> {
    let options = FormatOptions {
        indent_width: 2,
        line_width: 80,
    };
    let output = format("let x=1", &options)?;
    assert_eq!(output, "let x = 1\n");
    Ok(())
}
```

`FormatOptions::default()` supplies the CLI defaults. `FormatError` describes
invalid options, source syntax errors with byte offsets, or validation
failures. `UPSTREAM_REVISION` exposes the syntax revision.

## Preservation and validation

All non-whitespace token spellings and the interleaved order of tokens and
comments stay unchanged. Literal and comment contents remain opaque,
including documentation comments, quasiquotes, interpolation, and indentation
inside multiline strings. The formatter preserves layout line endings and
adds one final newline to nonempty source. Empty source remains empty.

Every returned candidate parses and has the same syntax tree after source
coordinates are excluded. The output is idempotent: formatting it again
produces identical bytes. This validates syntax without resolving imports,
checking types, executing Revo, or expanding procedural macros.

Formatting procedural macros is allowed. Macros that inspect offsets, lines,
or columns can observe formatting-induced position changes. The guarantee is
syntax equivalence modulo those coordinates, so such macros can produce
different results after formatting.

## Tested upstream corpus

The [vendored corpus provenance](tests/fixtures/upstream/PROVENANCE.md) lists
20 valid inputs from the same pinned revision: six complete `.rv` examples
and fourteen self-contained documentation snippets. It also lists one
malformed upstream documentation fence, which is tested as a syntax rejection.
The valid fixtures include the demo, pipes, procedural macros, types, control
flow, match arms, multiline literals and comments.

Each valid input is checked at line widths 24, 80 and 120 with indent widths
2 and 4, giving 120 input/option combinations. Each combination produces one
formatter result, checked for reparsing, exact
interleaved raw token/comment bytes, complete AST equivalence modulo coordinates,
and idempotence. Four of those same results also check reviewed expected output
for signature/table reflow, block indentation and match-arm layout. The private
preservation and corpus tests run once in library test modules; process-level
CLI checks remain integration tests.
Negative controls cover malformed sources, whitespace-sensitive calls,
comment movement and literal respelling. This is a bounded regression corpus;
it does not establish exhaustive syntax coverage or uniform layout quality.

See the [architecture follow-up](docs/verification/2026-10-06-architecture-followup.md) for
current verification and review status, and the [earlier architecture report](docs/verification/architecture-deepening.md)
for stage-specific package checks and measurements.

## Development and verification

Use exact Zig 0.17.0 on `PATH`, or export its absolute executable once:

```sh
export ZIG=/absolute/path/to/zig-0.17.0/zig
cargo test --all-targets
cargo test --doc
cargo clippy --all-targets -- -D warnings
cargo fmt --check
"$ZIG" fmt --check bridge.zig bridge/*.zig
"$ZIG" test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter indexed --test-filter 'input limits:' --cache-dir target/zig-test-cache
cargo build --release
(cd vendor/revo && sha256sum --check SHA256SUMS)
(cd tests/fixtures/upstream && sha256sum --check SHA256SUMS)
scripts/verify-editors
```

Editor checks require Neovim >=0.10, Node.js >=20, npm, and Python >=3.11.
The script uses `target/release/revofmt` by default; set `REVOFMT_BIN` to an
absolute executable path to use another build. VS Code's package README
documents dependency installation and VSIX packaging separately; its pinned
packaging tool requires Node.js >=22.

The [architecture follow-up](docs/verification/2026-10-06-architecture-followup.md)
records the latest editor checks: 27 Neovim, 49 VS Code and 15 Zed tests, plus
native VS Code host coverage and its limits. The
[original editor verification record](docs/verification/2026-10-06-editor-integrations.md)
retains the earlier package checks and reviews.

The latest source check passed 88 Rust tests (71 library, one binary, 16 CLI) and
nine filtered Zig tests. The Rust suite includes the 120-case corpus matrix;
these cases are not 120 additional test functions. The Zig filters select local
bridge, index and admission tests without running upstream runtime tests.
`cargo audit` is an optional additional dependency check when installed.

For a quick manual test, save this intentionally compact source as `example.rv`:

```revo
fn twoSum(nums,target) do
for y in 0..len(nums) do
for x in y+1..len(nums) do
if nums[y]+nums[x]==target do
return {y,x}
end
end
end
end
```

```sh
target/release/revofmt example.rv
# Reports a formatting difference and exits 1.
target/release/revofmt --check example.rv
# Applies the result; the following check exits 0.
target/release/revofmt --write example.rv
target/release/revofmt --check example.rv
# Try narrower output without modifying the file.
target/release/revofmt --line-width 24 example.rv
```

At default width, the result is:

```revo
fn twoSum(nums, target) do
  for y in 0..len(nums) do
    for x in y + 1..len(nums) do
      if nums[y] + nums[x] == target do
        return {y, x}
      end
    end
  end
end
```

For contributions, read [AGENTS.md](AGENTS.md), the [domain glossary](CONTEXT.md)
and [current architecture](docs/architecture.md). The [documentation index](docs/README.md)
separates current guides from completed plans and historical research/reviews.
