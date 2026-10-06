# Revo source formatting

Formatting changes whitespace around Revo source while retaining its lexical
contents and complete parsed structure modulo source coordinates.

This is the current domain glossary. See the [architecture guide](docs/architecture.md)
for module ownership and [README.md](README.md) for user-visible formatting rules.

## Language

**Token tape**:
The interleaved ordered sequence of token kinds and exact source bytes,
including comments and literals. Preservation compares this single tape.

**Opaque token**:
A literal or comment whose complete source bytes, including delimiters and internal newlines, are retained unchanged.

**Source region**:
A source-backed hint identifying a statement, block, expression scope, or
header/body seam. It is layout metadata; its extent may omit surface punctuation.

**Lexical envelope**:
The complete source-token extent of a construct, including the matching closer of every delimiter opened inside it.

**Block envelope**:
The complete extent of an actual source `do ... end` block, established by the
source collector from parsed provenance and completed descendants. Keyword
spellings used as names do not establish block ownership.

**Generated wrapper**:
A parser-created node without an independent source construct, such as a
lowered pipe binding. Real source blocks beneath it still need layout envelopes;
synthetic statement/header hints and opaque interiors stay excluded.

**Layout scope**:
A construct whose lexical envelope owns its internal layout and continuations. A
scope is usable only when its complete envelope fits within the current token
range. Match subjects and arm lists own separate continuations.

**Analyzed source**:
Source text paired with its token tape and source-region hints.

**Candidate**:
A proposed layout that has not yet passed source-preservation checks.

**Source preservation**:
Equality of token tapes together with equality of parsed trees after excluding source coordinates.
This is syntax preservation; programs and position-sensitive macros are not
executed to establish behavioral equivalence.

**Input admission**:
Lexer-only resource checks before parsing, recursive traversal or layout. Global
budgets include decoded interpolation and quasiquote fragments and apply again
to candidates. The [admission policy](docs/verification/input-limits.md) defines
the exact costs and verified limits.

**Fixed point**:
A validated result whose complete formatting operation produces identical
bytes. The formatter returns only a fixed point, with at most four passes.

**Conservative layout**:
A layout retaining original line-boundary and semantic adjacency decisions when preferred reflow cannot preserve source.

**Header**:
The condition, range or signature preceding a body. Its compactness is measured
independently of that body.

**Continuation**:
The remainder of an expression placed below its initial line. Flat operators share one indentation level; actual nested constructs introduce their own levels.

## Editor integration

**Adapter**:
An editor package that recognizes Revo files, sends the whole unsaved buffer to
the installed CLI, and applies a successful current result. It delegates syntax,
layout and preservation to the formatter.

**Buffer representation**:
The editor's text and line-ending model. Neovim and VS Code check that applying
CLI output preserves its bytes; Zed's native pipeline limits supported
preservation to LF source. See the [editor guides](editors/README.md).

**Stale result**:
Output from a request superseded by another request or a changed, closed or
unrepresentable buffer. The adapter or native editor discards it.
