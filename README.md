**This project is making heavily use of AI Agents, if you have a problem with that just don't use it. Thanks!**

# `revofmt`, a formatter for revo

a command line tool and rust library for [revo](https://github.com/if-not-nil/revo).
it formats your code, checks that it parses the same way, and leaves
literal and comment bytes alone. formatting the result again gives the same bytes.

[install](#install) | [how to use](#how-to-use) | [editors](#editors) | [build from source](#build-from-source) | [reference](docs/formatter.md)

## install

These steps install the ready-made Linux download. You don't need Rust, Zig,
or Revo installed.

The download works on **Linux with an Intel or AMD 64-bit processor**
(`x86_64`), with glibc 2.34 or newer and `libgcc_s`. There are no supported
macOS, Windows, ARM, or Alpine Linux downloads yet.

If you use Neovim, [the Neovim plugin](https://github.com/w0x7y/revofmt.nvim)
can download and verify revofmt for you. For the command line, follow these steps.

1. **Download the files.** Create a folder named `revofmt` inside your Downloads
   folder. Click each link below and save all five files in that folder.

   - [revofmt-linux-x86_64-gnu](https://github.com/w0x7y/revo-formatter/releases/download/v0.1.1/revofmt-linux-x86_64-gnu): the program.
   - [SHA256SUMS](https://github.com/w0x7y/revo-formatter/releases/download/v0.1.1/SHA256SUMS): checks that the downloads are complete and unchanged.
   - [LICENSE](https://github.com/w0x7y/revo-formatter/releases/download/v0.1.1/LICENSE): the formatter license.
   - [REVO-LICENSE.txt](https://github.com/w0x7y/revo-formatter/releases/download/v0.1.1/REVO-LICENSE.txt): the Revo license.
   - [THIRD_PARTY.md](https://github.com/w0x7y/revo-formatter/releases/download/v0.1.1/THIRD_PARTY.md): third-party notices.

   Keep the filenames as shown above. If your browser displays a text file,
   right-click its link and choose **Save link as**.

2. **Install revofmt.** Open your Terminal application and paste this block.
   If you saved the files elsewhere, replace the folder on the first line.

   ```sh
   cd "$HOME/Downloads/revofmt" &&
     sha256sum --check SHA256SUMS &&
     install -Dm755 revofmt-linux-x86_64-gnu "$HOME/.local/bin/revofmt"
   ```

   You should see `OK` next to each of the four checked files. The command then
   copies the program to `~/.local/bin/revofmt`. It does not need `sudo`.
   Keep the download folder for its license notices.

3. **Try it.** Paste this into the same terminal.

   ```sh
   export PATH="$HOME/.local/bin:$PATH"
   revofmt --version
   printf 'let x=1' | revofmt
   ```

   The version line starts with `revofmt 0.1.1`. The last command should print:

   ```revo
   let x = 1
   ```

### If something goes wrong

- **`revofmt: command not found` in a new terminal:** run the `export PATH`
  command from step 3 again. To make it permanent, add that line to `~/.bashrc`
  if you use Bash, or `~/.zshrc` if you use Zsh, then open a new terminal.
  If you use Fish, run `fish_add_path ~/.local/bin` once instead.
  `PATH` is the list of folders your terminal searches for commands.
- **`No such file or directory` during installation:** check that the download
  folder and all five filenames match step 1. If your Downloads folder has a
  different name, change the path in step 2.
- **A checksum says `FAILED`:** download that file again before installing.
- **`GLIBC_... not found` or `Exec format error` when running revofmt:** check
  the Linux and processor requirements above.

You can now [format your Revo files](#how-to-use) or
[set up your editor](#editors). Building from source below is optional.

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
- [vs code](https://github.com/w0x7y/revofmt-vscode): install the VSIX, then use Format Document.
- [zed](https://github.com/w0x7y/revofmt-zed): install the formatting language server, then run `editor: format`.

all three format the whole unsaved buffer. format-on-save is opt-in.
setup, source, packaging and editor tests live in those repositories.
the [integration guide](docs/editors.md) describes the CLI contract and downstream checks.

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
`e94e6d89ddaabb3249b38c1b10df87c700d1e8dc`.
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
```

To check complete programs with a separately built upstream compiler at the
vendored revision, run:

```sh
REVO_BIN=/absolute/path/to/revo/zig-out/bin/revo scripts/check-upstream
```

This checks compiler acceptance, execution results before and after formatting,
idempotence at four widths, and the range/interpolation rejection boundaries.
It also records a compiler-only range error that the parse-only formatter
intentionally accepts.

keep both debug and release tests when changing recursive paths or input limits.
editor checks run in their [dedicated repositories](docs/editors.md#verification)
against the rebuilt formatter.

for an offline rust package check with dependencies cached, run
`cargo package --offline --allow-dirty` with the same `ZIG` setting.
`cargo audit` is an optional dependency check.

before contributing, read [AGENTS.md](AGENTS.md), the [glossary](CONTEXT.md) and
[architecture](docs/architecture.md). the [documentation index](docs/README.md)
links current guides and dated verification records.

## credits

[MIT](LICENSE). the parser comes from [revo](https://github.com/if-not-nil/revo),
also MIT; its notice and provenance are in [THIRD_PARTY.md](THIRD_PARTY.md).
