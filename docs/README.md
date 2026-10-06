# Documentation

## Current guides

- [Project README](../README.md): build, CLI/library use, formatting rules and testing.
- [Contributor instructions](../AGENTS.md): contracts and change workflow.
- [Domain glossary](../CONTEXT.md): source and layout vocabulary.
- [Architecture](architecture.md): implemented flow and module ownership.
- [Editor integrations](../editors/README.md): package layout, shared CLI contract, installation and checks.
- [Input admission policy](verification/input-limits.md): exact resource budgets.
- [Third-party provenance](../THIRD_PARTY.md): compiler pin, vendor closure and licenses.
- [Corpus provenance](../tests/fixtures/upstream/PROVENANCE.md): attributed inputs and reviewed outputs.
- [Architecture follow-up verification](verification/2026-10-06-architecture-followup.md): current source/editor checks, block ownership, launcher fix and review status.

## Plans and original designs

These describe decisions and implementation stages. Earlier private interfaces,
task sequencing and workspace details may have been superseded; use the current
guides above for new work.

| Stage | Design | Implementation plan |
| --- | --- | --- |
| Initial formatter | [Original design](superpowers/specs/2026-10-05-revo-formatter-design.md) | [Completed plan](superpowers/plans/2026-10-05-revo-formatter.md) |
| Initial architecture deepening | [Original design](superpowers/specs/2026-10-05-architecture-deepening-design.md) | [Completed plan](superpowers/plans/2026-10-05-architecture-deepening.md) |
| Formatting rules | User examples and reproduced regressions | [Completed plan](superpowers/plans/2026-10-05-formatting-rules.md) |
| Layout scopes and document fitting | Findings recorded in the plan | [Completed plan](superpowers/plans/2026-10-06-layout-depth.md) |
| Editor integrations | [Approved design](superpowers/specs/2026-10-06-editor-integrations-design.md) | [Implementation plan](superpowers/plans/2026-10-06-editor-integrations.md) |
| Architecture follow-up | [Approved design](superpowers/specs/2026-10-06-architecture-followup-design.md) | [Completed plan](superpowers/plans/2026-10-06-architecture-followup.md) |

## Historical verification

Recorded counts and benchmarks apply to the tree reviewed at each stage. Later
reports do not retroactively rerun or replace those measurements.

- [Bridge build](verification/bridge-build.md)
- [Initial whole-project review](verification/whole-project-review.md) and [match-arm fix review](verification/match-arm-fix-review.md)
- [Initial formatter verification](verification/2026-10-05-v0.md)
- [Initial architecture deepening verification](verification/architecture-deepening.md)
- [Formatting rules verification](verification/2026-10-06-formatting-rules.md)
- [Layout deepening verification](verification/2026-10-06-layout-depth.md)
- [Final source check before the architecture follow-up](verification/2026-10-06-final-check.md)
- [Editor integration verification](verification/2026-10-06-editor-integrations.md)
- [VS Code activation follow-up](verification/2026-10-06-vscode-activation.md)

## Pinned upstream research

Research describes the revisions inspected on 2026-10-05. Claims about upstream
tools or ecosystem availability are dated observations.

- [Original formatter research](../research/revo-formatter.md)
- [Syntax inventory](../research/syntax-inventory.md) and [independent review](../research/syntax-inventory-review.md)
- [Parser architecture research](../research/parser-architecture.md) and [independent review](../research/parser-architecture-review.md)
- [Research corrections re-review](../research/research-rereview.md)
