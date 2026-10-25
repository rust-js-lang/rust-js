# 0858. A comment at a body's end is there, before its `None` or after its last code

Status: Accepted. Extends [0753](0753-a-branch-of-only-comments-keeps-them.md),
whose block of only comments kept them where it ends, to a block whose
comments come after its last code, and to a function's `None` given at its
end, which is no `return;`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): comments, which
nothing runs.

## Context

React's `countChildren` says why its callback gives nothing:

```js
mapChildren(children, () => {
  n++;
  // Don't return anything
});
```

Its port gives the `None` the callback's type asks for, with the comment
before it:

```rust
&|_, _| {
    n.set(n.get() + 1);
    // Don't return anything
    None
}
```

A comment is its next statement's (ADR 0444), and the `None` a function
gives at its end is a `return;` that isn't written: it went, and its
comment with it. So did a comment after a body's last code, `out.push(0);
// Nothing to do.`, which ADR 0753 kept only of a block of nothing else.

## Decision

- **A function's `None` given at its end, a comment before it, is
  `StmtKind::Comments` where the `None` is**, which lowering makes as it
  makes MIR's `_0 = None` (`sources::commented`); the `return;` goes, and
  the comment is printed where it was, before the body's `}`.
- **A block whose comments come after its last code ends with them**, as a
  block of only comments does (`sources::comments_end`): `out.push(0); //
  Nothing to do.` at the function's or the branch's end.
- **A function's body reaches its comments**, as a block's does: its span,
  to them, so oxc prints them inside its `}` (`commented_end`, one answer
  for both).

```js
export function found(n) {
  if (n > 3) {
    return n;
  }
  // Nothing found.
}
```

## Why

- **It's React's JS**, its comment where it wrote it.
- **A comment is where its code is (ADR 0444)**: here, the end of a body.

## Alternatives

- **`return; // Don't return anything`**: a `return` the program didn't
  write, which rust-js leaves out (ADR 0299).

## What holds it

- `test/lowering-comments.test.ts`, "a comment before a None given at a
  function's end, or after its last code, is at its end": a closure's, a
  function's after an early `return`, and a branch's and a function's
  after their last code, each run.
- Mutations `tail-none-comments-dropped`
  (`scripts/mutations/lower/mir/assign.ts`), `comments-end-only-comments`,
  `commented-never` (`lower/sources.ts`) and
  `function-body-comments-outside` (`to_oxc/statements.ts`).
