# Scoped re-review of research corrections

Historical scoped re-review of research corrections from 2026-10-05. Verdicts
and line references below describe the reviewed versions; they do not report a
new build or runtime check. Implementation has since completed. Use the
[documentation index](../docs/README.md) for current instructions and verification,
and the [architecture](../docs/architecture.md) for delivered interfaces.

Reviewed 2026-10-05. Comparison: `f06d5126` to `c90b6c1d`. Scope was the two changed research reports, the original independent reviews, and the relevant preservation, analysis-interface and build sections of the existing design and implementation plan. The accepted contract permits formatting-induced source-coordinate changes, including coordinates observable by proc macros.

The repository's Graft graph was reported empty, so the concrete source checks used read-only excerpts from `/tmp/revo-formatter-upstream`. Those checks were limited to the new record-field fixture, comment byte ranges and `Node.synthetic_block`. I did not repeat remote revision/download-index queries, runtime macro execution, bridge compilation or the broader syntax research. This report is the only file written by this review; no code or Git state was changed.

## Research quality verdict

Pass for this correction scope. All six named findings are addressed. The revised reports distinguish syntax preservation from behavioral invariance, define the printer-facing analysis interface, and keep build feasibility as an unproven implementation gate. No new correction defect was found.

## Requirement compliance verdict

Pass. The corrections retain the requested Rust library and CLI, first-release width reflow, exact literal/comment preservation, pinned frontend validation and non-destructive failure. The macro policy matches the user's explicit acceptance; refusing to format coordinate-observing macros is not an additional requirement. The proposed formatter still must parse without expanding macros or resolving imports.

## Per-finding verdicts

| Original finding | Verdict | Evidence and scope |
| --- | --- | --- |
| Exact frontend toolchain pin | Addressed | `syntax-inventory.md:5` names the stable Zig 0.17.0 release, the Linux x86_64 archive and SHA-256, and requires corresponding host checksums. `parser-architecture.md:99` rejects development and other versions. The design at lines 87-92 and plan at lines 87, 103 and 107 carry the pin into build work. This verifies the documented pin, not a new download or build. |
| Missing record-field doc attachment | Addressed | `syntax-inventory.md:75-88` adds named, optional and positional field attachment, trimming, last-doc-wins behavior, and a preservation fixture. Its comparator requirement at line 124 and `parser-architecture.md:68` retain docs and optionality. The fixture and claims agree with `type_syntax.zig:191-225` and `ast.zig:56-62`. |
| Metadata outside `Expr`, particularly `synthetic_block` | Addressed | `syntax-inventory.md:124` explicitly retains `Node.synthetic_block` and explains its top-level/grouped-import and prelude-flattening role. `parser-architecture.md:68` and plan line 106 retain the flag. Excluding synthetic layout annotations does not authorize excluding this field from structural comparison. |
| Macro-observable nested spans | Addressed under the accepted policy | `parser-architecture.md:9`, `68` and `76-86`, plus `syntax-inventory.md:130-142`, document the nested-span serializer path and the original 86/88 witness. They explicitly limit the guarantee to syntax equivalence modulo positions and permit the witnessed coordinate change. Design lines 52-57 and plan lines 23 and 153 agree. No full behavioral-equivalence claim is needed for the accepted contract. |
| Missing token/region analysis contract | Addressed | `parser-architecture.md:62-66` links to plan Task 1 and specifies interleaved tokens, complete raw UTF-8 byte ranges, `statement`/`block`/`match_arm` regions, owned JSON buffers and explicit freeing. Plan lines 52-85 and 104-108 define the Rust data/result interfaces and ownership. Research line 64 acknowledges incomplete desugared spans; plan lines 145 and 148 assign concrete grouping to Rust source-token analysis. Task 1 requires independent interface review before Task 2, so this is a defined implementation boundary rather than a claim that region extraction already works. |
| CRLF comments contain a raw CR | Addressed | `parser-architecture.md:51`, design lines 76-78 and plan line 153 require preserving that CR and appending only LF. The research and plan request exact-byte regression coverage. This agrees with `Lexer.zig:631-632`, which stops before LF and consumes a preceding CR. |

The original architecture review also warned that separate comment and code-token lists permit comment movement. `parser-architecture.md:50` and `62`, design lines 76-77 and plan line 150 now explicitly require one interleaved `(kind, raw bytes)` sequence. That clarification is addressed too.

## New correction defects

None found in the scoped changes. Actual bridge linking, exhaustive comparison, region extraction, raw-range recovery and formatter regressions remain implementation work. Their absence during this research correction does not reverse either verdict.
