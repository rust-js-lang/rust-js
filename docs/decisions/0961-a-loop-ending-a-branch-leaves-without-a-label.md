# 0961. A loop that ends a branch of the body it ends leaves with no label

Status: Accepted. Extends `outer_continues` ([0702](0702-a-while-is-its-test-and-its-break-where-written.md)'s loops
ending another's body) and `tail_continues` ([0432](0432-a-return-is-written-where-its-value-is-made.md),
[0684](0684-a-continue-the-program-wrote-is-where-it-wrote-it.md)) to what React's test
selectors write; a reversed range down to 1 is `i > 0`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each rewrite leaves the
same jumps: a turn of the outer loop ends where the inner loop is left.

## Context

React's `hasMatchingPaths` and `findAllNodes` walk a fiber's children at
the end of a loop's body, in the `else` of a test that returns:

```js
if (selectorIndex === selectors.length) {
  return true;
} else {
  let child = fiber.child;
  while (child !== null) { stack.push(child, selectorIndex); child = child.sibling; }
}
```

Their port printed `bb7: while (..)` with `while (true) { if (child == null)
{ continue bb7; } .. }`: a JS label where upstream has none, never an
accepted port difference. `outer_continues` made a `continue l` of a loop
that ends `l`'s body its `break`, but only of a loop that's the body's last
statement, not one that ends a branch of an `if` that is. And
`findBoundingRects`'s inner `for`, whose body is an `if` of `else if`s each
ending in `break`, kept a `continue;` at its end, which the settled stage's
`innermost jumps` unnamed after `tail continues` had run; and `(1..n).rev()`
was `i >= 1` where upstream writes `i > 0`.

## Decision

- **A loop that ends a branch of an `if` that ends loop `l`'s body**, as one
  that ends the body itself, leaves by `break` where it wrote `continue l`
  (`outer_continues`, `ending`): either way `l`'s turn ends there. Then
  `early breaks` and `while tests` make it `while (child != null)`.
- **`tail_continues` runs again after the settled stage's `innermost
  jumps`**: a `continue` those passes unnamed where its loop's turn ends
  anyway is dropped, as one named was.
- **A reversed range's loop down to 1 tests `i > 0`**, down to any other
  start `i >= start`: for integers the two are one test, and `> 0` is how
  a person counts down to 1.

## What holds it

- `test/corpus/loop_ending_branch_loop.rs`, against native Rust: a
  fiber walk's `while (child != null)` with no label, `for (let i = n - 1;
  i > 0; i--)`, and React's merge of bounding rects with no `continue`.
- Mutations: `outer-continues-of-a-branch-unseen`
  (`scripts/mutations/lower/mir/tidy/jumps/continues.ts`),
  `settled-tail-continues-kept` (`stages.ts`),
  `reversed-range-to-one-at-least-one` (`loops.ts`).

## Not decided here

`findBoundingRects`'s `boundingRects[i]` and `boundingRects.splice(i, 1)`
stay `$index` and `$removeAt`: the index is in bounds only by an invariant
of the loop (each turn removes at most one item, at `i`, then `i` steps
down), which the ranges don't know of a `Vec` changed in the loop. Its
`else if`s, each ending in `break`, print as `if`s.

## Amendment: the settled stage's `innermost jumps` goes (2026-10-29)

The nightly's mutation `mir-tidy-headed-labels-kept`, which took the
settled stage's `innermost jumps` away, survived the whole corpus with
its snapshots and the lowering, snapshot, JSX, port, compiler and
tidy-stage suites: by then no tested program's jump names the loop it's
in, so the pass changed nothing. It goes with its mutation; `tail_continues` still runs
after `outer continues`, and no label comes back into any program's JS.
The `loops` stage's pass stays: without it a label nothing names reaches
the `chains` stage, whose `each label named` contract refuses it
(`mir-tidy-innermost-jumps-labeled`, caught by
`test/corpus/match_breaks_its_loop.rs`).
