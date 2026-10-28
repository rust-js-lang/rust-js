# 0658. A comment before a branch's value is before it in the conditional

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
[0621](0621-comments-around-macros-and-at-a-modules-end.md) and
[0630](0630-a-comment-inside-an-expression.md); amends
[0657](0657-an-if-of-one-value-each-is-a-conditional.md), whose branch with
a comment kept its `if`. Amended: a comment after a branch's value is
right after it.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments, which
nothing runs.

## Context

React's `reportGlobalError` says why it takes the host's function, in its
branch:

```js
const reportGlobalError =
  typeof reportError === 'function'
    ? // In modern browsers, reportError will dispatch an error event,
      // emulating an uncaught JavaScript error.
      reportError
    : error => { .. };
```

Its port has the comments in the `if`'s branch, above
`webapi::global::report_error`. A branch with a comment kept its `if`
(`tidy::commented`), "which an expression has no line for", and in a
`thread_local!` even that lost it: the `if`'s span is the macro's, so a
comment anywhere in it kept every branch of it an `if`, and the one
before a value had no statement to be before, so it was lost; one before
a closure, `|n| ..`, went inside its body.

## Decision

**A comment before a branch's value, where only the start of the value
is between them, is before it in the conditional, on lines of its own,
as Prettier lays one out: `? // In modern browsers ..` and the value on
the next line.** So a branch with a comment is folded as any branch is
(ADR 0657), and `tidy::commented`, and the comments it read, are gone.

- **A conditional's branch is a start**, as a statement is (ADR 0444,
  rule 1): what a comment is before. A statement starting where it does
  is the statement's, as before.
- **The printer writes it**: oxc writes no comment before a
  conditional's first branch, so each branch's comments are written
  into oxc's output, the line split after its `?` or `:`, where the
  branch's first placed JS is, or a line before, an arrow's `(` being
  placed by none; the map moved to match (`format::moved_positions`), and
  oxfmt lays it out.
- **The same in a function and a module**: a `let`'s, a `return`'s and a
  thread-local's are each this conditional.

## Why

- **Every comment of the Rust is in the JS** (ADR 0444), here where it's
  in React's.
- **One form of an `if` of one value each** (ADR 0657): a comment is no
  reason for a second.

## Alternatives

- **The comment before the whole statement**: kept, but not where it's
  about, as React's isn't.
- **The `if` kept where a branch has a comment**: two forms of one value,
  and the comment lost in a macro's.

## Consequences

- **A comment after a branch's value, on a line of its own before its
  `}`**, is nobody's, as before: no start is after it.

## What holds it

- `test/node-bindings.test.ts`, "React's reportGlobalError is a
  conditional with its comments": the port as written, its JS React's
  `const` of a conditional with both comments, run in Node, with and
  without the host's `reportError`.
- `test/lowering-names.test.ts`, "a comment is in the JS where it's in
  the Rust": a comment before each branch, an `if`'s and a `match`'s.
- Mutations `arm-comment-unplaced`, `arm-comment-after-question-only`.

## Amendment: a comment after a branch's value

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments.

ReactFiberApplyGesture writes `nextPhase = CLONE_UNHIDE; // TODO: Mark the
name and find a pair.` in an `if`. Its port, as clippy's
`needless_late_init` has it, `let next_phase = if .. { // .. CLONE_UNHIDE
// TODO: .. } else { visit_phase };`, printed the conditional with the
`// TODO` above the loop it was in: a comment after a branch's value,
before its `}`, is before no start, and went to the innermost statement it
was inside, the loop.

**The user's decision: the conditional stays, and a line comment on a line
of its own after a branch's value, before its block's `}`, is right after
that value, as if written after it on its line:**

```js
const nextPhase =
  phase === 1
    ? // This was an Enter of a ViewTransition.
      2 // TODO: Mark the name and find a pair.
    : phase;
```

`trailing::own_line_after_value` finds it, past whitespace and other
comments each way, by where each conditional's branch's value ends
(`arm_starts`' `?` and `:`), and gives it to the comments after code on
their line (ADR 0444), whose line comment ends its JS's line, the rest on
the next, `? 2 // .. \n : now` (ADR 0733). The statement form upstream
writes, `let next; if (..) { next = ..; // TODO }`, would be a second form
of one `if` (ADR 0657), which a comment is no reason for. A block comment
there, or a comment after code that's no branch's value, is placed as
before.

`test/lowering-comments-ends.test.ts` "a comment after a branch's value
is right after it" (ApplyGesture's loop, a function's tail of two
branches); mutations `after-value-not-at-close` and `after-value-any-code`.
