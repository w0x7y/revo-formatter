# Formatter final check

Reviewed the complete feature against `683d5c2a` (then the branch's `HEAD`,
`main`, and `origin/main`), including all unstaged changes and relevant untracked
plans/verification notes. No feature commits or staged changes were present.
The Git index was left unchanged. Earlier stage verification counts are historical;
this record describes the source state after this review. Documentation was
subsequently refreshed for integration; the current references are
[the project README](../../README.md), [contributor instructions](../../AGENTS.md),
[architecture](../architecture.md), and [documentation index](../README.md).

## Coverage

| Changed files | Reviewed contracts |
| --- | --- |
| `CONTEXT.md`, `README.md` | Domain glossary, approved formatting rules, public options, preservation and compatibility claims |
| `bridge/source.zig` | Lexical endpoints, opaque/generated exclusions, indexed arrows, header/body and match hints, arena ownership |
| `src/document.rs` | Cached widths, suffix reservation, list/operand fitting, indentation, Unicode columns, CRLF handling |
| `src/layout.rs`, `src/layout_index.rs` | Complete scope containment, scope priority, statement boundaries, generics, unary/index spacing, operator chunks and comments |
| `src/oracle.rs` | Source metadata contract, malformed-response handling, token/AST preservation and source-region regression assertions |
| `src/tests/formatting.rs` | Literal output assertions, boundary/composition cases, preservation and idempotence; changed expectations agree with the approved policy |
| `tests/fixtures/upstream/expected/docs-ambient-24-4.rv`, `PROVENANCE.md`, `SHA256SUMS` | Golden output matches the formatter; descriptions and manifest agree with maintained sources |
| `docs/superpowers/plans/2026-10-05-formatting-rules.md`, `2026-10-06-layout-depth.md` | Authorized scope, completed tasks and actual implementation |
| `docs/verification/2026-10-06-formatting-rules.md`, `2026-10-06-layout-depth.md`, this record | Historical evidence, current review coverage and verification limits |

Surrounding code and consumers were traced through the document/layout interface,
public formatter, source oracle, CLI, bridge frontend, admission checks, comparator,
build script, and corpus/CLI tests. No confirmed dead imports, exports, dependencies,
unreachable branches, debug leftovers, or redundant maintained files needed removal.

## Findings fixed

1. **A long call expanded a short generic list.** At width 24,
   `f<T>(first_argument,second_argument)` split `<T>` across three lines because
   the generic document reserved the whole call's flat width. Generic calls now
   attach only their opening parenthesis to the generic list. Enclosure contents
   fit independently after that opening renders, so both a short generic list
   before long arguments and short arguments after a long generic list stay
   compact. Empty calls reserve both parentheses. A failing public formatter
   regression covered calls, named function signatures, and the width-20 empty
   call boundary before the fixes; it now passes with literal output, preservation,
   and idempotence checks.
2. **Corpus metadata lagged the approved formatting changes.** The ambient golden
   checksum failed `sha256sum --check`, and provenance still claimed expression
   bodies were unindented and pipes were never reflowed. Updated the descriptions
   and regenerated the manifest from its existing file list. Only the provenance
   and already-changed ambient golden digests changed; upstream input bytes and
   licenses remain unchanged.

## Security review

Inspected the runtime entry points and trust paths: bounded UTF-8 CLI reads,
public source/candidate admission before recursive work, private synchronous FFI
calls and buffer freeing, frontend JSON/range validation, opaque source handling,
pure parser use without import resolution or runtime execution, and file write
prevalidation, create-new temporary files, permissions, atomic replacement and
failure cleanup. Build commands use argument vectors and a trusted local `ZIG`
selection; the build downloads no source or tools. The maintained repository has
no network server, authentication, deployment, or CI configuration to review.

`cargo audit --json` checked all 13 locked Rust dependencies against RustSec
(database commit `ef6173cbc5c50ec8166f9a5b28f07834144373ee`, updated 2026-10-03).
It reported zero advisories and zero warnings. A filename-only credential-pattern
scan of maintained sources, tests, and docs found no matches and exposed no secret
values. No confirmed security defect was found in these inspected paths.

Coverage limits: dedicated `gitleaks` and `cargo-deny` scanners were not installed.
The secret check was a heuristic, not an exhaustive scanner. The unchanged vendored
frontend was reviewed at its active lexer/parser and bridge entry points, and its
full checksum manifest was verified; unused compiler/runtime modules were not
individually security-audited. Verification targets the documented native Linux
platform; unsupported hosts/targets were not tested. No live exploitation,
credential changes, publishing, external source scanning, or production actions
were performed.

## Final verification

After the last source edit, using the pinned Zig 0.17.0 executable:

- `cargo test --all-targets`: 60 library, one binary and 16 CLI tests passed,
  including all 120 corpus/option combinations and 2 MiB stack/admission checks.
- `cargo test --doc`: passed (zero doctests).
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo fmt --check` and `zig fmt --check bridge.zig bridge/*.zig`: passed.
- `zig test bridge.zig -lc -O ReleaseSafe --test-filter 'bridge:' --test-filter indexed --test-filter 'input limits:'`: nine bridge/index/admission tests passed.
- `cargo build --release`: passed.
- Five exact release CLI layouts passed, including the original twoSum example,
  long/short generic-call compositions, an empty-call boundary, and a block suffix.
  Each output was idempotent and returned zero with no output from `--check`.
- A release CLI composition sweep passed 220 cases across five widths and four
  indentation settings, including generic signatures, nested scopes, comments,
  chained calls and following statements. Every output was idempotent.
- Both upstream-corpus and vendor checksum manifests passed.
- `cargo audit --json`: passed with zero advisories/warnings.
- Final tracked/cached diffs and new untracked text files passed whitespace checks.

No remaining actionable findings in the inspected scope. All changed files are
accounted for above; no required project check remained unavailable.

## Documentation refresh before integration

Updated all maintained Markdown documents on 2026-10-06. Added root `AGENTS.md`,
`docs/README.md` and `docs/architecture.md`; expanded the README with complete
verification commands and the twoSum print/check/write walkthrough. Corrected
corpus test paths, full local Zig test selection and the bridge report's log
location. Marked completed plans and older research/reviews as historical while
retaining their original measured counts, commit references and findings.
Upstream source, fixture inputs and license notices remain unchanged.

Fresh integration checks passed on the complete feature tree: all 77 Rust tests,
zero doctests, all nine local Zig tests, clippy, Rust/Zig formatting and a release
build. Both checksum manifests passed after refreshing the corpus provenance
digest. All 28 maintained Markdown files passed fenced-block and local-link
checks (125 local links); `git diff --check` passed. The README's exact twoSum
output, initial check exit 1, write, subsequent check exit 0 and narrow-width
idempotence were verified through the release CLI in a temporary directory.
