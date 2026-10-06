# Architecture follow-up design

The user requested an architecture scan followed by implementation of its
findings, without a visual report or a candidate-selection step. The review
starts at `aea81c06` and weights recent editor and layout changes.

## Findings and scope

The editor adapter modules already have useful depth. Their small interfaces
hide process lifecycle and buffer representation knowledge; the deletion test
would move that complexity back into callers. No shared editor runtime or
arbitrary file consolidation is justified. The CLI's complete batch preparation
and atomic replacement also already have coherent ownership.

Two concrete findings warrant changes:

1. The native VS Code test launcher observes exit codes but ignores signal
   termination. A signaled launcher waits until the generic 60-second deadline.
   Correct this local lifecycle rule and test the actual launcher interface.
2. Block ownership is inferred from raw `do`/`end` spelling in both the bridge
   token index and the Rust layout index. Revo permits these spellings as names,
   including fields and bindings. A name can prematurely close a block or open
   a false block, changing indentation and statement layout.

## Deeper block ownership

The source collector owns actual block lexical envelopes, using the parsed
tree and its existing source-backed traversal. Complete nested envelopes before
resolving an enclosing block's final `end`, since AST spans can stop before a
closing delimiter and declaration spans can omit descendants.

The collector's existing `block` source regions carry the complete envelopes.
The Rust layout index consumes those facts for block pairing instead of
reinterpreting keyword text. Ordinary punctuation pairing remains lexical.
This puts the semantic rule at one seam, improves locality, and gives all layout
callers leverage from the same verified facts. Keep metadata shape and public
formatter/editor interfaces unchanged.

Use existing source filtering for generated and opaque descendants. Do not
introduce another parser, a grammar keyword exclusion list, a new shared runtime,
or a new externally exposed test hook.

Generated wrappers must not hide the facts for actual source-backed blocks
nested inside them. Preserve those block envelopes for layout, while keeping
synthetic statement/header hints and opaque descendants excluded.

## Verification and contracts

- Keep token/comment tape equality, complete AST equality modulo coordinates,
  opaque literals/comments, and byte-identical idempotence for returned results.
- Keep parser-sensitive adjacency and existing fitting rules.
- Apply the existing resource admission before parsing, traversal and generated
  candidate verification. Do not change admission budgets, vendor source,
  upstream/toolchain pins or the verified platform.
- Preserve CLI exit codes, complete write-batch prevalidation and atomic file
  replacements; preserve editor settings and commands.
- Write focused failing formatter cases with literal expected output and actual
  preservation/idempotence checks before changing block ownership.
- Test the native launcher through a real subprocess: signaled and nonzero
  failure are prompt and informative; successful early wrapper exit still waits
  for the host's atomic result rather than passing immediately.
- Run package checks, native VS Code checks, full README verification and a fresh
  independent review of each implementation plus the final change range.
- Keep existing user editor configuration untouched. No publication or push.

## Evidence limits

Existing editor representation limits remain in force, including Zed's LF-only
preservation scope and Neovim's native EOL undo limitation. The new block tests
add concrete syntax coverage; they do not claim exhaustive grammar coverage.
