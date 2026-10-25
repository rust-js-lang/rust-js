# 0630. A comment inside an expression is after the code before it

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
whose rule 2 was of a comment at its line's end. Amended below: one after
an `=` is before the value; amended again: and one on a line of its own
between an `=` and its value.

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

## Amendment: a comment after an `=` is before the value

React's reconciler lines its bit flags up with a comment between each
`=` and its value, in ReactTypeOfMode.js, ReactHookEffectTags.js,
ReactFiberFlags.js and ReactFiberLane.js, hundreds of times:

```js
export const NoMode = /*                         */ 0b0000000;
// TODO: Remove ConcurrentMode by reading from the root tag instead
export const ConcurrentMode = /*                 */ 0b0000001;
```

The port writes the same line, `pub const NO_MODE: TypeOfMode = /* .. */
0b0000000;`, which rustfmt keeps. The code right before the comment is
the `=`, no name, number or string, so the nearest token before it was
the declaration's, and the comment was after `export`: `export /* .. */
const NoMode = 0b0000000;`, `const /*  */ mode = ..` of a `let`.

**A block comment right after an `=`, a declaration's or an
assignment's, with code after it on its line, is before the value: after
the `=` of the JS of its line**, its text as written, spaces and all:

```js
export const NoMode = /*                         */ 0b0000000;
const mode = /*  */ (given | ConcurrentMode) >>> 0;
last = /* again */ (last | ((mode + 1) >>> 0)) >>> 0;
let count = /* none yet */ 0;
```

- **Which `=`**: the one before the JS the map places of the code nearest
  after the comment on its line, the value's; where there's none before
  it, the one after it, the statement's JS being placed where the value
  is, `let last = /* first */ mode` of `let mut last`; and where the
  value's code has no JS, a thread local's `const { Cell::new(0) }`, the
  one after the declaration's JS. A module's `const`, a `static`, a
  thread local, a `let` and an assignment are all written so.
- **Only `=`**: a comment after `==`, `!=`, `<=`, `>=` or a compound
  assignment, `+=`, is after the code before it, as before, `a /* the
  other */ === b`. Where its line's JS has no ` = `, it's as before too.
- **After the value, a line comment ends the line**, `export const
  Removed = 0b01; // Removed`, unchanged.
- **The source map moves past each comment inside a line**, this one or
  `37 /* "%" */`: the JS after it is its width further along, where its
  tokens had stayed where they were before the comment was added.

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments, which
nothing runs.

Test: `lowering-comments`'s "a comment between an = and its value is
before the value": ReactTypeOfMode's lines, a `let`, a `let mut`, an
assignment, a `static`, a thread local and a comment after `==`, run, and
the map's columns after a comment. Mutations
`value-comment-after-declaration`, `value-comment-after-comparison`,
`inside-comment-map-unmoved` (`to_oxc.ts`) and
`value-comment-equals-only-before` (`to_oxc/trailing.ts`): where a comment
after code on its line goes, which this made `to_oxc.rs` too long to
hold, is `src/to_oxc/trailing.rs`'s.

## Amendment: one on a line of its own between an `=` and the value

ReactFiberFlags.js writes one mask's comment where Prettier breaks its
line:

```js
export const ViewTransitionNamedStatic =
  /*    */ SnapshotStatic | MaySuspendCommit;
```

rustfmt writes the port's `= /*    */ SNAPSHOT_STATIC | ..` too long for
its line with the comment on a line of its own:

```rust
pub const VIEW_TRANSITION_NAMED_STATIC: Flags =
    /*    */
    SNAPSHOT_STATIC | MAY_SUSPEND_COMMIT;
```

A comment on its own line is before the statement after it (ADR 0444),
but this one is inside a module's item, whose `const` no statement
starts, so it was lost: `export const ViewTransitionNamedStatic =
SnapshotStatic | MaySuspendCommit;`.

**A block comment of one line, alone on its line, whose code before it,
past other comments, is an `=` (not `==`, `<=` or `+=`), and whose code
after it is the value on a line after it, is before the value**, placed as
one on the value's line right before its first code is, after the `=` of
the JS (`to_oxc/trailing.rs`'s `own_line_before_value`), and not before a
statement too. rust-js prints it on one line, `export const
ViewTransitionNamedStatic = /*    */ SnapshotStatic | MaySuspendCommit;`;
an app's Prettier, React's, breaks it after the `=`, as upstream's is. A
line comment there, `==\n // the other\n other`, is as before, before the
statement or in its expression where oxc puts it.

Test: `lowering-comments`'s "a comment on its own line between an = and
its value is before the value", a module's `const` and a `let`, run.
Mutations `own-line-value-comment-unplaced`,
`own-line-value-comment-also-before-statement` (`to_oxc.ts`).
