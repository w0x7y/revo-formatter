# Revo source formatting

Formatting changes whitespace around Revo source while retaining its lexical contents and parsed meaning modulo source coordinates.

This is the current domain glossary. See the [architecture guide](docs/architecture.md)
for module ownership and [README.md](README.md) for user-visible formatting rules.

## Language

**Token tape**:
The ordered sequence of token kinds and exact source bytes, including comments and literals.
_Avoid_: Separate code and comment streams

**Opaque token**:
A literal or comment whose complete source bytes, including delimiters and internal newlines, are retained unchanged.

**Source region**:
A source-backed hint identifying a statement, block, expression scope, or header/body seam. Its extent may omit surface punctuation.
_Avoid_: Complete syntax tree

**Lexical envelope**:
The complete source-token extent of a construct, including the matching closer of every delimiter opened inside it.

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
_Avoid_: Runtime behavioral equivalence

**Conservative layout**:
A layout retaining original line-boundary and semantic adjacency decisions when preferred reflow cannot preserve source.

**Header**:
The condition, range or signature preceding a body. Its compactness is measured
independently of that body.

**Continuation**:
The remainder of an expression placed below its initial line. Flat operators share one indentation level; actual nested constructs introduce their own levels.
