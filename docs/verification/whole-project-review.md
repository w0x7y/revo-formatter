# Whole-project independent review

Base: `c90b6c1d616c52c4ff7a5fab8338a12e235a6cbc`
Head: `a2c139a532dcb5f3f7b39953a3e78720df1e21ff`

## Strengths

- The implementation follows the approved architecture and scope: Rust owns formatting and the CLI; a separately maintained bridge statically incorporates the unchanged pinned frontend. Compiler version, supported host/target and provenance are explicit. Fresh controller evidence includes an offline source-package extraction/build, so required vendored inputs are verified beyond the development checkout.
- `bridge/compare.zig:6` compares every supported field by type and contents, excluding only exact `ast.Span`. `src/lib.rs:59` combines this with one interleaved raw token/comment tape; neither AST equality alone nor separate comment preservation is mistaken for the full guarantee. Owned bridge buffers have a Rust drop guard, and each parse uses a fresh arena.
- `src/lib.rs:26` returns only verified formatting fixed points. The preferred/conservative choice cannot emit invalid intermediate text. Literal envelopes, CRLF comments and source-coordinate policy are handled explicitly.
- `src/cli.rs:204` validates the complete write batch before replacing any file. Same-directory create-new temporary files, original permissions, content synchronization, atomic replacement, error cleanup and completed-path diagnostics cover the agreed mutation contract.
- Tests exercise actual frontend and CLI processes. The 20 valid corpus inputs and one explicit rejection, 120 option combinations, four golden outputs and negative preservation controls support the documented bounded claims. README accurately acknowledges current layout and platform limits.

## Issues

### Critical

None found.

### Important

1. **Match-arm ranges exclude closing delimiters, breaking block and table indentation.**

   Locations: `bridge/source.zig:107`, `src/layout.rs:187`, `src/layout.rs:200`.

   The bridge bounds each match arm with `arm.then.span.end`, although the design and oracle contract explicitly recognize that AST spans can omit surface punctuation. The layout builder accepts a delimiter pair only when its closing token falls inside the current sequence. In a match-arm body, omitted closing tokens therefore prevent the body from being recognized as a complete delimited group; the closing `end` or `}` prints in the enclosing sequence, and nested block indentation can also collapse.

   Reproduced with the current release binary and default options, exit code 0:

   Input:

   ```revo
   match x
   | _ => do
   f()
   end
   ```

   Output:

   ```revo
   match x
     | _ => do
       f()
   end
   ```

   The closing `end` is at column zero, outside its arm. An ordinary unlabelled outer block reproduces the same error one level deeper:

   ```revo
   do
     match x
       | _ => do
         f()
     end
   end
   ```

   The defect is broader than closing-keyword alignment. With `match x\n| _ => do\ndo\nf()\nend\nend`, the inner `f()` receives the same indentation as its opening `do`, and both closing keywords are at column zero. With `match x\n| _ => {\nx=1,\ny=2\n}`, the closing brace is also at column zero.

   Syntax safety remains intact, but block/arm indentation is an explicit first-version requirement. Existing corpus safety assertions cannot detect incorrect indentation; current goldens do not include this combination.

   Fix arm layout boundaries using complete lexical delimiter envelopes, including nested blocks and tables; source tokens must remain authoritative when AST spans omit punctuation. Ensure the resulting indentation composes correctly with the arm-body indentation rather than simply extending a range and introducing an extra level. Add expected-output regressions for a direct block arm, a nested block arm, a table arm, multiple arms and an arm inside an unlabelled outer block. Retain tape, AST and idempotence assertions.

### Minor

- Previously identified region lookup complexity remains: `bridge/source.zig:49`, `:61`, `:65` and `:91` repeatedly scan the token tape while walking AST nodes. The resulting quadratic work is an acknowledged deferred performance limitation, not a new release blocker for this bounded v0.

## Review and verification scope

Read the approved design and implementation plan, the complete original-source/provenance/fixture review diff, task-review finding summaries and the fresh controller verification record. Immutable vendor source/license files were accepted on the independently verified 23-file exact-byte/hash evidence. No vendor rewrite, full-suite rerun, runtime macro execution or network lookup was performed.

Controller evidence records 63 Rust test executions (45 distinct tests), three Zig tests, Rust/Zig formatting, clippy with warnings denied, release build, 13 smoke cases without runtime Zig/Revo and offline extracted-package verification. Focused release-CLI probes in this review covered mixed opaque/layout endings, a doc envelope following multibyte source, a binary continuation beside a comment, and the match-arm delimiter cases above. Twelve distinct probe inputs all exited successfully; the first four were also rerun on their output and were stable. The formatting defect is a layout-quality failure rather than a validation or process failure.

Graft was used for focused navigation after reading the diff. Its reported source-read savings totaled approximately 3,305 tokens. This review changed only this report.

## Assessment

**Ready to merge: With fixes.**

The build, ownership, preservation and CLI boundaries are sound in the reviewed change, and the verification evidence is strong for the documented v0 scope. Fix the match-arm delimiter/indentation defect and independently review its focused regression coverage before completion; no other Critical or Important findings were found.
