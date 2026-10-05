# Revo Formatter Architecture Deepening Implementation Plan

> **For agentic workers:** Use subagent-driven-development to implement this plan task by task, with an independent spec/quality review after each task and a final whole-change review. User explicitly requested implementation without visual-report or candidate-selection phases.

**Goal:** Deepen the layout and preservation modules and fix all four demonstrated architectural findings.

**Architecture:** LayoutIndex owns complete source-token envelopes and continuation bounds; the Zig collector owns indexed token lookups; AnalyzedSource owns paired text/metadata and the preservation contract. Tests cross the real preservation interface rather than compiling private implementations repeatedly.

**Tech Stack:** Rust 2024, existing Cargo dependencies, Zig 0.17.0 and immutable pinned Revo frontend.

**Spec:** docs/superpowers/specs/2026-10-05-architecture-deepening-design.md

## Global Constraints

- Preserve public format(source: &str, options: &FormatOptions) -> Result<String, FormatError>, defaults indent 2/width 80, bounds indent 1..=8/width 20..=240 and all CLI exit/write contracts.
- Preserve exact interleaved token bytes, full parsed structure excluding only ast.Span, and formatting idempotence. No candidate may escape unvalidated.
- Source-coordinate effects on procedural macros remain accepted; no runtime execution or import resolution.
- Revo pin b571298b6fc95bc863548f118354c8d077792f6f, Zig exact 0.17.0, immutable vendor and native x86_64-unknown-linux-gnu scope remain unchanged.
- Do not add dependencies, features, public inspection interfaces, mock adapters or build downloads.
- No visual report, candidate-choice gate, speculative parser/renderer/CLI rewrite or invented remote metadata.
- Work only in /home/idan/GitRepo/revo-formatter/.worktrees/architecture-deepening. Never edit primary checkout product files.
- Build with ZIG=/home/idan/GitRepo/revo-formatter/.tools/zig-x86_64-linux-0.17.0/zig and CARGO_TARGET_DIR=/home/idan/GitRepo/revo-formatter/target supplied explicitly.

## File map

- src/layout_index.rs: private complete-envelope and indexed source-layout facts.
- src/layout.rs: document construction and continuation indentation.
- bridge/source.zig: token lookup index and unchanged metadata collection interface.
- src/oracle.rs: analyzed source, FFI ownership and complete preservation checks.
- src/lib.rs: public options and formatting orchestration.
- src/tests/{mod,formatting,corpus}.rs: private-library behavior/preservation tests without oracle recompilation.
- tests/cli.rs: unchanged actual-process integration contract.
- README.md and docs/verification/: measured current architecture/verification notes.

### Task 1: Deepen layout indexing and fix expression continuations

**Files:** Create src/layout_index.rs. Modify src/layout.rs, src/lib.rs only for private module declaration, and tests/formatting.rs for meaningful regressions. Preserve document renderer unless a concrete additional defect is reproduced and reported.

**Interfaces:** Consumes existing Analysis { tokens, regions }, SourceToken { kind, start, end } and source text. Produces private LayoutIndex::new(source: &str, analysis: &Analysis) -> Self with delimiter_close(open: usize) -> Option<usize>, arm_end(bar: usize) -> Option<usize>, statement_end(token: usize) -> Option<usize>, and generic_angle(token: usize) -> bool. The layout function interface remains unchanged in this task. Task 3 will change constructor input to paired AnalyzedSource and adapt direct callers.

- [ ] Add a focused failing expected-output regression before fixes:

```rust
let options = FormatOptions { line_width: 24, ..FormatOptions::default() };
check("let x=first_argument+do\nf()\nend\nlet y=2",
      "let x = first_argument +\n  do\n    f()\n  end\nlet y = 2\n", options);
```

Add nested do, table and parenthesized RHS followed by a statement at widths 24/80 and indentation 2/4. Include multiple arms, outer blocks and comments to prove envelopes do not capture a closer opened outside their region or swallow following source. Every expected-output case retains tape/AST/idempotence evidence.

- [ ] Add flat-chain RED controls:

```rust
let options = FormatOptions { line_width: 24, ..FormatOptions::default() };
let source = format!("let x={}", std::iter::repeat_n("value", 8).collect::<Vec<_>>().join("+"));
let output = format(&source, &options).unwrap();
assert!(output.lines().count() > 1);
assert!(output.lines().skip(1).all(|line| line.bytes().take_while(|b| *b == b' ').count() == 2));
let large = format!("let x={}", std::iter::repeat_n("1", 800).collect::<Vec<_>>().join("+"));
let formatted = format(&large, &FormatOptions { line_width: 20, ..options }).unwrap();
assert!(formatted.len() <= large.len() * 8);
assert_eq!(format(&formatted, &FormatOptions { line_width: 20, ..options }).unwrap(), formatted);
```

Also cover mixed operators/precedence, delimited operands, nested lists/blocks, comments, original hard breaks and adjacent statements/arms. Record actual focused failures; do not add timing thresholds.

- [ ] Construct delimiter pairs once. Normalize statements and arms with one complete lexical-envelope operation. Use binary searches for region starts and an active-end event sweep for the minimum eligible statement end at each token, handling overlapping hints. Keep generic preservation behavior. Place these invariants behind LayoutIndex methods; avoid exposing raw parallel arrays to document construction.
- [ ] Make each flat binary segment own one continuation indentation, with existing useful width-dependent grouping and actual nested constructs introducing their own scope. Keep opaque bytes, source line breaks, conservative mode and safety validation unchanged. Add exact snapshots of chosen chain output after the policy is established.
- [ ] Run focused red/green, full Rust suite once after final changes, fmt/clippy, and inspect output growth at 100/200/400/800 operands. Growth assertion is structural, not wall-clock. Commit and write task report. Independent review gates Task 2.

### Task 2: Deepen Zig source-token indexing without changing metadata

**Files:** Modify bridge/source.zig and add focused metadata tests in src/oracle.rs or local bridge tests. A private index may remain in source.zig rather than create a trivial wrapper file. No vendor/comparator/frontend protocol changes.

**Interfaces:** Preserve tokens(alloc, source, lexed) and regions(alloc, source, lexed, root) return values, ordering and coordinate semantics. Collector owns a private TokenIndex initialized once with lexed tokens. Sorted searches provide exact-start, exact-end and containment checks; one do/end pass supplies concrete block closers. No new Rust-facing method or adapter.

- [ ] Capture baseline metadata for all 20 valid pinned corpus fixtures before modifying collection. A temporary probe can serialize token/region kind/start/end triples; record a digest plus exact before/after equality in the report. Preserve fixture notices and original source bytes.
- [ ] Add behavior tests for nested/labeled blocks, match bars with intervening comments, grouped imports, quasiquotes, interpolation descendants, empty source, Unicode docs/module envelopes and generated synthetic nodes. Verify metadata counts/extents through the existing oracle interface. If fixing a newly demonstrated metadata defect, reduce it to a focused RED first and report the intentional baseline difference.
- [ ] Build a private token lookup index once. Replace repeated whole-tape scans in endpoint checks, grouped-import detection, do/end matching, opaque containment and arm-bar lookup with indexed queries. Compute concrete block end once per statement. Keep raw lexer span semantics distinct from complete output token envelopes.
- [ ] Preserve overlapping/empty endpoint behavior including EOF, and exact generated/opaque exclusions. Index initialization can allocate through the operation arena; no new global mutable state or native-handle lifecycle.
- [ ] Compare all baseline corpus metadata exactly. Add generated many-statement/many-arm cases and one-off before/after measurements at 200/400/800 statements outside test assertions. Verify behavior and code-level lookup complexity without flaky timing tests.
- [ ] Run relevant Rust oracle tests, direct Zig tests, Rust/Zig formatting and clippy. Run complete suite once after final changes, commit and write actual evidence report. Independent review gates Task 3.

### Task 3: Deepen analyzed-source preservation and replace duplicated tests

**Files:** Modify src/oracle.rs, src/lib.rs, src/layout.rs, src/layout_index.rs. Move private-inspection formatting/corpus tests into src/tests/{mod,formatting,corpus}.rs and register ordinary cfg(test) library modules. Keep tests/cli.rs as integration processes. Update README and a new concise docs/verification/architecture-deepening.md record with actual final counts.

**Interfaces:** Replace loose analysis production consumption with:

```rust
pub(crate) struct AnalyzedSource<'a> {
    source: &'a str,
    analysis: Analysis,
}
pub(crate) fn analyze(source: &str) -> Result<AnalyzedSource<'_>, FormatError>;
impl<'a> AnalyzedSource<'a> {
    pub(crate) fn source(&self) -> &'a str;
    pub(crate) fn tokens(&self) -> &[SourceToken];
    pub(crate) fn regions(&self) -> &[SyntaxRegion];
    pub(crate) fn preserves(&self, candidate: &str) -> Result<bool, FormatError>;
}
```

AnalyzedSource layout input replaces separate source/Analysis arguments. LayoutIndex::new consumes &AnalyzedSource<'_>. Constructor validates token ranges once for ordered, disjoint, nonempty bounds and UTF-8 slicing. Region ranges require bounded UTF-8 slices; preserve permitted empty/overlapping/unordered region semantics. Oracle equivalent remains internal for low-level structural controls; no new public interface or native handles.

- [ ] Add focused preservation-interface controls before moving policy: same tape/different AST, equal AST/moved comments, respelled literals, syntax-invalid candidates, valid candidates, CRLF/Unicode/opaque contents and propagated bridge failures. Add private malformed-metadata constructor tests for reversed/out-of-bounds/overlapping token ranges and invalid UTF-8 endpoints; regions may overlap but must have valid bounded slices.
- [ ] Move candidate analysis, syntax-invalid rejection, exact tape comparison and AST equality into AnalyzedSource::preserves. Source parse errors retain original diagnostics; malformed bridge/schema/allocation failures remain Validation errors. Validate options before analyzing. Keep fixed-point limit/fallback/error behavior and FFI ownership unchanged.
- [ ] Adapt layout and its index to the paired analyzed source. Callers cannot combine unrelated source text and metadata. Remove obsolete lib.rs preservation policy/helpers after callers move; do not introduce a pass-through facade or test-only public method.
- [ ] Move all tests needing private oracle into library test modules importing the real crate oracle. Remove #[path = "../src/oracle.rs"] recompilation and duplicate local raw-tape implementations. Use one formatter result per valid fixture/option to check preservation plus idempotence and expected output. Preserve every old behavior/negative control and all Task1/2 additions. Tests of the preservation seam must prove both tape and AST predicates independently with complementary controls; do not weaken the contract for convenience.
- [ ] Keep process-level CLI checks; preserve external public-interface usability. Update include paths without relying on temporary checkout locations. Retain all attributed fixtures/provenance. Report distinct tests, executed counts and the unchanged 120-case corpus matrix honestly after deduplication.
- [ ] Run focused controls, full Rust/Zig suite, Rust/Zig fmt, clippy, release build and CLI smoke with Zig unavailable at runtime. Verify package inputs and extracted-source build, including new Rust module files and excluding .worktrees/.superpowers/local tools/cache. Commit and report; independent task and final whole-change review required.

## Review and completion

Use a plan-scoped ignored ledger for task commits/reviews/fix rounds. Controller independently reviews discovery against source and concrete probes. Implementers do not spawn agents. Fresh task review checks compliance and quality; original implementer owns fixes, scoped re-review gates each fix. Finish with a whole-change review, fresh covering verification and a permanent concise record. Do not publish a new feature or silently remove tests/corpus fixtures.
