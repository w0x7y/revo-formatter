**This project is making heavily use of AI Agents, if you have a problem with that just don't use it. Thanks!**

# `revofmt`, a formatter for revo

a command line tool and rust library for [revo](https://github.com/if-not-nil/revo).
it formats your code, checks that it parses the same way, and leaves
literal and comment bytes alone. formatting the result again gives the same bytes.

[get](#get) | [how to use](#how-to-use) | [editors](#editors) | [build from source](#build-from-source) | [reference](docs/formatter.md)

## get

[download a release](https://github.com/w0x7y/revo-formatter/releases).
the current binary is for linux x86_64 GNU, with glibc >=2.34 and `libgcc_s`.
you don't need rust, zig or a revo installation to run it.

```sh
# download the binary, checksum and license notices into an empty directory
mkdir revofmt-download
cd revofmt-download
for file in revofmt-linux-x86_64-gnu SHA256SUMS LICENSE REVO-LICENSE.txt THIRD_PARTY.md; do
  curl --fail --location --output "$file" \
    "https://github.com/w0x7y/revo-formatter/releases/download/v0.1.0/$file"
done

# check it, then install it
sha256sum --check SHA256SUMS && \
  install -Dm755 revofmt-linux-x86_64-gnu "$HOME/.local/bin/revofmt"
"$HOME/.local/bin/revofmt" --version
```

make sure `~/.local/bin` is on your PATH before using `revofmt` below.
if you're here for neovim, [the plugin](https://github.com/w0x7y/revofmt.nvim)
can download and verify the formatter for you.

## how to use

```sh
printf 'let x=1' | revofmt
# let x = 1

revofmt example.rv                       # print it, leave the file alone
revofmt --check first.rv second.rv       # check without writing
revofmt --write first.rv second.rv       # format in place
revofmt --indent-width 4 example.rv      # default: 2 spaces
revofmt --line-width 24 example.rv       # default: 80 columns
revofmt -- --example.rv                  # a filename starting with a dash
revofmt --help
```

with `--line-width 24`, this:

```revo
print(first_argument, second_argument)
```

becomes:

```revo
print(
  first_argument,
  second_argument
)
```

width is a soft target. comments, literals and syntax that can't safely break can
go past it. indentation accepts 1 to 8 spaces; width accepts 20 to 240 columns.

exit codes are `0` for success, `1` when `--check` finds a difference, and `2` for
an error. diagnostics go to stderr. invalid syntax produces no formatted output.

`--write` validates every input before changing any file. each replacement is
atomic; the whole batch isn't. it rejects stdin, symlinks and nonregular files.
there's no directory discovery or config file yet.

## editors

- [neovim](https://github.com/w0x7y/revofmt.nvim): install with lazy.nvim, then `:RevoFormat`.
- [vs code](editors/vscode/README.md): install the extension package, then use Format Document.
- [zed](editors/zed/README.md): install the local extension and configure `revofmt` as the formatter.

all three format the whole unsaved buffer. format-on-save is opt-in.
the [editor guide](editors/README.md) has setup and verification commands.

## build from source

you need rust with edition 2024 support and **zig 0.17.0**.
the verified build platform is native linux x86_64 GNU; other host/target
combinations are rejected. builds never download tools or source.

```sh
git clone https://github.com/w0x7y/revo-formatter.git
cd revo-formatter

zig version                            # must be exactly 0.17.0
cargo build --release
printf 'let x=1' | target/release/revofmt
```

if zig isn't on your PATH, point `ZIG` at its executable:

```sh
ZIG=/absolute/path/to/zig-0.17.0/zig cargo build --release
```

the frontend is linked into the binary. zig is only needed to build it.
[THIRD_PARTY.md](THIRD_PARTY.md) records the toolchain pin and vendored source.

## in rust

use this checkout as a dependency:

```toml
[dependencies]
revofmt = { path = "/absolute/path/to/revo-formatter" }
```

```rust
use revofmt::{FormatError, FormatOptions, format};

fn main() -> Result<(), FormatError> {
    let output = format("let x=1", &FormatOptions::default())?;
    assert_eq!(output, "let x = 1\n");
    Ok(())
}
```

## the limits

supported syntax is pinned to revo revision
`b571298b6fc95bc863548f118354c8d077792f6f`.
validation compares token/comment bytes and the syntax tree, ignoring source
coordinates. it doesn't resolve imports, check types or execute macros.
a macro that reads source positions can observe formatting changes.

source and generated results have resource limits: 262,144 UTF-8 bytes, 4,096
expanded lexer tokens, and 32 combined delimiter/embedded-source levels, plus
shared recursive budgets. see the [input policy](docs/verification/input-limits.md)
for the exact rules, and the [reference](docs/formatter.md) for layout and preservation details.

## development and verification

use exact zig 0.17.0. from the repository root:

```sh
export ZIG=/absolute/path/to/zig-0.17.0/zig
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
scripts/verify-editors
```

keep both debug and release tests when changing recursive paths or input limits.
editor checks need neovim >=0.10, node.js >=20, npm and python >=3.11.
`REVOFMT_BIN` can select an absolute formatter path; otherwise the script uses
`target/release/revofmt`. the [vs code guide](editors/vscode/README.md#development-and-verification)
covers native host checks and packaging, which needs node.js >=22.

for an offline rust package check with dependencies cached, run
`cargo package --offline --allow-dirty` with the same `ZIG` setting.
`cargo audit` is an optional dependency check.

before contributing, read [AGENTS.md](AGENTS.md), the [glossary](CONTEXT.md) and
[architecture](docs/architecture.md). the [documentation index](docs/README.md)
links current guides and dated verification records.

## credits

[MIT](LICENSE). the parser comes from [revo](https://github.com/if-not-nil/revo),
also MIT; its notice and provenance are in [THIRD_PARTY.md](THIRD_PARTY.md).
