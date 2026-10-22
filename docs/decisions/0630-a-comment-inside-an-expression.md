# 0630. A comment inside an expression is after the code before it

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
whose rule 2 was of a comment at its line's end.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments, which
nothing runs.

## Context

React's `normalizeConsoleFormat` names each character code it tests:

```js
if (formatString.charCodeAt(i) !== 37 /* "%" */) { .. }
switch (formatString.charCodeAt(++i)) {
  case 79 /* "O" */:
  case 99 /* "c" */:
```

A comment after code on its line was after the JS of the last line that
code maps to (ADR 0444): one with more code after it on its line, inside
an expression, was at the end of another line, `return 1; /* "O" */ /*
"c" */`, or above the next statement.

## Decision

**A block comment with code after it on its line is after the JS of the
code right before it**: the source map's token of that line nearest
before it, a name, a number or a string, and the comment after its text,
`37 /* "%" */`; the nearest whose JS is one, where another token maps
from the same place. A `case` is where its pattern's constant is written
(`Constant::Value`), and its `switch` where its `match` is (`Switch`'s
span), not its first case's constant, which a source map holds one token
of: so each comment follows its constant, `case 79 /* "O" */:`. A comment
the map has no such token for is at its line's end, as before, never
lost.

## Why

- **A comment is where its code is** (ADR 0444): one inside an
  expression is about the code beside it.

## What holds it

- `test/lowering-names.test.ts`, "a comment inside an expression is
  after the code before it".
- Mutations `inside-comment-at-line-end`, `case-constant-unplaced`,
  `switch-at-first-case`.
