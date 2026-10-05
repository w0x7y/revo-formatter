# Revo source formatting

Formatting changes whitespace around Revo source while retaining its lexical contents and parsed meaning modulo source coordinates.

## Language

**Token tape**:
The ordered sequence of token kinds and exact source bytes, including comments and literals.
_Avoid_: Separate code and comment streams

**Opaque token**:
A literal or comment whose complete source bytes, including delimiters and internal newlines, are retained unchanged.

**Source region**:
A source-backed hint identifying a statement, block or match arm. Its extent may omit surface punctuation.
_Avoid_: Complete syntax tree

**Lexical envelope**:
The complete source-token extent of a construct, including the matching closer of every delimiter opened inside it.

**Analyzed source**:
Source text paired with its token tape and source-region hints.

**Candidate**:
A proposed layout that has not yet passed source-preservation checks.

**Source preservation**:
Equality of token tapes together with equality of parsed trees after excluding source coordinates.
_Avoid_: Runtime behavioral equivalence

**Conservative layout**:
A layout retaining original line-boundary and semantic adjacency decisions when preferred reflow cannot preserve source.

**Continuation**:
The remainder of an expression placed below its initial line. Flat operators share one indentation level; actual nested constructs introduce their own levels.
