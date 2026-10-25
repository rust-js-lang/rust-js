# 0753. A branch of only comments keeps them

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md):
a comment with no statement after it in its block is printed there.
Supersedes its consequence that an `else` of only comments is no `else`.
Amended: a branch a function's end empties is no branch either (below). Extended by
[0862](0862-a-comment-at-a-bodys-end-is-there.md): a block's comments after its last
code, and a function's before the `None` it gives at its end.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments, which
nothing runs.

## Context

React's `addObjectDiffToProperties` (shared/ReactPerformanceTrackProperties.js)
says why a branch does nothing:

```js
if (indent >= 3) {
  // Just fallthrough to print the two values if we're deep.
  // This will skip nested properties of the objects.
} else if (/* .. */) {
```

A comment is its next statement's (ADR 0444), and this branch has none.
MIR has no statement of it either, only the jump past the chain, so tidy
saw `if (indent >= 3) { break bb77; }`, took it for an `if` of nothing and
turned the chain, `if (indent < 3) { if (..) { .. } }`, the comments gone.
Decided with the user (2026-10-24): comments aren't nothing.

## Decision

- **A block of only comments is a statement, `StmtKind::Comments`,
  where it ends**: lowering makes one of the `()` MIR assigns of the block,
  whose span is the block's (`sources::comments_end` says whether it holds
  only comments, and where its `}` is). Nothing runs of it; tidy keeps it,
  so the branch is one of something, as it's written.
- **The printer writes its comments there**: they're attached to it, and,
  no node starting where it is, given to oxc as comments whose node is
  gone, which it prints before what follows, or before the block's `}`,
  to which the block's span reaches. `if (indent >= 3) { // .. } else if`.
- A branch with no comment, `else {}`, is nothing, and is still left out.

## Why

- **It's upstream's JS**, the chain as it's written and the comments that
  say why its first branch is empty.
- **A comment is where its code is (ADR 0444)**: here, the end of a block.

## Alternatives

- **The comments before the `if`**: they'd say why of what isn't there.
- **An empty statement, `;`, holding them**: JS no person writes.

## Consequences

- `test/lowering-comments.test.ts` holds a branch's and an `else`'s; the
  port's `addObjectDiffToProperties` has upstream's chain.

## Amendment: a branch a function's end empties is no branch

React's `enqueueTask`, as react-in-rust ports it, ends with an `if let`
with no `else`, in the function `js::export_default!` holds:

```rust
if let Some(enqueue_task_impl) = ENQUEUE_TASK_IMPL.get() {
    enqueue_task_impl(task);
}
```

It printed `if (!enqueueTaskImpl) {} else { enqueueTaskImpl(task); }`, a
`then` of nothing, no comment, which no person writes. Not a regression of
this decision: `git bisect` found it at every commit that compiles the
shape, back to when `export_default!` first held a function (f3afc81c),
and of any `macro_rules!` that holds one, back to da4b2a24 at least.

MIR's `return` that a macro's function ends with has the macro's span, and
rust-js places a macro's span at the macro's call (ADR 0444). That is
before the arm, so `written_first` (in `tidy.rs`) makes the arm the `if`'s
`else` and leaves a `return;` in its `then`. `tail_returns` then removed
the `return;` from each branch, which nobody wrote, and kept the `if` as it
was, with a `then` of nothing. It now gives the `if` to
`branches::emptied_branch`, the one owner of a branch left with nothing,
as `shared_tails` and `branch_values` do: `if (enqueueTaskImpl) { .. }`. A
`then` of only comments still has its `StmtKind::Comments` and is kept.

Held by `test/lowering-early-exits.test.ts`'s "an if let a macro's function
ends with is no empty then and an else", which compiles and runs the port's
shape, and mutation `mir-tidy-tail-return-emptied-then-kept`. No other
`{} else` is in the corpus's, the snapshots' or the guide's JS.
