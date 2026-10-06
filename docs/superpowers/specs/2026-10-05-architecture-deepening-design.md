# Formatter architecture deepening

Historical approved design/implementation plan; implementation and its review gates are complete. Original interface sketches and stage-specific instructions below are retained as design history. Use the [current architecture](../../architecture.md) for delivered module boundaries and the [final source check](../../verification/2026-10-06-final-check.md) for verification.

The user requested implementation of concrete architecture improvements, explicitly skipping the visual report and candidate-selection/grilling loop. Discovery identified four actionable findings in the recently changed layout and preservation modules. No CONTEXT.md or ADR existed; CONTEXT.md now records the resolved source-formatting language.

## Scope and invariants

Preserve the public formatter interface and all existing option/default/error/CLI contracts. Rust owns layout and user-facing formatting; the pinned upstream frontend remains the syntax authority. Retain the exact ordered interleaved token tape, complete structural equality excluding only ast.Span, and idempotence. Never execute Revo programs, macros or imports. Source-coordinate effects in procedural macros remain accepted. Vendor bytes, parser/toolchain pins and native platform scope remain unchanged.

## Layout depth and locality

A private LayoutIndex module establishes delimiter pairs, complete statement/arm lexical envelopes, generic-angle protection and innermost per-token statement ends before document construction. This concentrates the source-region invariant behind a small internal interface. Extracting raw arrays alone would not add depth; construction must absorb the incomplete-span repair and repeated-search obligations.

Statements and match arms use the same lexical-envelope operation. An opener inside a region brings its matching closer into that region; an opener outside cannot donate its closer. Complete ranges must not swallow a following statement or arm. Event-based statement indexing preserves the minimum eligible end for overlapping hints, avoiding assumptions that all regions form a perfect tree.

Flat binary segments own one continuation indentation. Subsequent operators can add fitting groups and soft breaks without growing indentation. Delimited operands, nested blocks and arm bodies start real indentation scopes. This fixes demonstrated truncated-RHS indentation and quadratic whitespace growth while retaining parser validation and existing useful width behavior.

## Source-metadata lookup depth

A private Zig token index is built once per region collection. Sorted-offset searches answer exact-start/end and containing-token questions; one pass records do/end pairs. Collector operations use this interface rather than repeatedly scanning every token. Preserve emitted metadata values/order, generated-node exclusions and opaque quasiquote/interpolation handling. This is an internal concrete module, not an interchangeable-adapter framework.

## Preservation seam and test locality

The oracle owns AnalyzedSource, a borrowed source string paired with owned token/region metadata. Construction verifies range ordering, bounds and UTF-8 slice validity once. Layout accepts that value rather than separately supplied strings/metadata. AnalyzedSource::preserves hides candidate parsing, exact tape equality, full structural comparison and syntax-invalid candidate rejection; bridge/validation failures remain errors.

Tests needing private oracle behavior become library test modules using the real oracle. Remove duplicate #[path] oracle compilation and duplicated tape-policy assembly. Corpus checks reuse one formatted result per fixture/option. Retain all expected-output, idempotence, provenance, malformed-source and complementary tape/AST negative controls; these verify the preservation interface includes both predicates. Keep actual CLI process integration tests.

## Rejected expansion

The renderer, comparator, CLI and build modules already provide useful depth. No general CST, complete token-kind mirror, filesystem adapter framework, native parse handles, alternative parser or new public feature is justified. Renderer fit caching is only in scope if a focused post-fix measurement demonstrates remaining concrete quadratic work. A renderer-only probe confirmed quadratic repeated suffix walks after continuation repair (about 3.8 million flat-width visits for 800 operands). Immutable grouped documents will cache flat width inside the renderer module; construction computes it once from already-grouped children. This preserves width decisions and hides cache consistency from callers. No visual HTML artifact or selection/approval gate is required by the user's instruction.

## Verification and reviews

Each task has focused regressions or metadata-equivalence evidence and an independent spec/quality review. Final checks include full Rust/Zig tests, style/lint, release build, real CLI smoke, package inputs and an extracted-source build. Preserve a concise final verification record, report measured test counts after deduplication, and update current README architecture/coverage notes without rewriting historical review records.
