# Match-arm fix review

Historical scoped review from 2026-10-05 of `a2c139a5..a0822758`. File/line
references below identify that diff; completion is recorded in the
[initial formatter verification](2026-10-05-v0.md). Use the
[documentation index](../README.md) for current instructions and the
[architecture](../architecture.md) for delivered envelope ownership and test paths.
The [later source/editor check](2026-10-06-editor-final-check.md) records subsequent
repairs, checks and coverage limits.

- **Match-arm delimiter envelopes and indentation — ADDRESSED.** `src/layout.rs:81` extends each arm's hinted token interval through every complete lexical delimiter pair opened within that arm, including nested block/table/call pairs. Since only openers inside the arm participate, the enclosing block's closer remains outside. `src/layout.rs:209` lets inline bodies share the arm's base indentation while retaining an extra level for bodies whose first code token follows a newline, including comment-separated bodies. Together these changes fix both omitted closers and redundant indentation.
- **Covering regression evidence checked:** `tests/formatting.rs:286` and `tests/formatting.rs:363` add 18 exact-output cases for direct/nested block arms, table/call arms, multiple arms, an unlabelled outer block, narrow/wide reflow, and next-line/comment-separated bodies. The existing helper also checks complete AST equality, interleaved raw tape and idempotence. The fix report names the covering tests and supplies their failing/passing output, the full formatting binary result, and subsequent full checks; those claims agree with the diff. No suite was rerun for this review because the recorded cases answer the specific finding and the code raises no additional unresolved doubt.
- **New breakage in the fix diff — None found.** Envelope extension is monotonic and bounded by existing pair indices; indentation selection uses original source boundaries without editing opaque text or bypassing validation.
- **Out-of-scope observations — None.**
- **Fix round — All findings addressed, no new Critical/Important breakage.** Ready to complete after the controller's planned fresh final verification. Reviewed only `a2c139a5..a0822758`, its report and the original finding; product source and Git state were not changed.
