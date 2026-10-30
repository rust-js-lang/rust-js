# 0753. A branch of only comments keeps them

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md):
a comment with no statement after it in its block is printed there.
Supersedes its consequence that an `else` of only comments is no `else`.
Amended: a branch a function's end empties is no branch either (below). Extended by
[0862](0862-a-comment-at-a-bodys-end-is-there.md): a block's comments after its last
code, and a function's before the `None` it gives at its end. Amended: a `break;` rust-js writes after them is after them.

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

## Amendment: a `break;` after them is after them (2026-10-28)

React port gaps ag4 and ag3 (ReactFiberApplyGesture's `case HostPortal:
{ // TODO: .. break; }`) and finishConcurrentRender's guarded arm, `case
RootSuspendedWithDelay: if (..) { // Commit the placeholder. break; }`.
A `match` arm of only comments is a case or a branch whose `break;` rust-js
writes, a case's by the printer (`case_body`), a branch's by tidy, with no
Rust of its own: the comments, given to oxc as comments whose node is
gone, were printed before what followed the `break;`, the next case's
first statement, or past the `break;` before the block's `}`.

**A statement rust-js made right after a block's comments is where they
end**, so oxc prints them before it: `case 4: // TODO .. break;`, `if (..)
{ // Commit the placeholder. break; }`, as upstream writes them
(`push_stmts`, `case_body`).

- Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.
- Held by `test/lowering-comments-ends.test.ts`, "an arm of only comments
  keeps them before its break", run; mutations `case-break-before-comments`
  and `made-after-comments-unplaced` (`to_oxc/statements.ts`).

## Amendment: a last arm of only comments keeps them in its clause (2026-10-28)

React's WorkLoop handleThrow ends a `switch` with a clause of only a
comment; its port's `_ => { // .. }`, the `default:` JS falls out of,
lost it, or printed it after the function (React port gap ag4b): a
case's comments are printed before the `break` rust-js ends it with, or
the next clause, and the last has neither, its place none. **The last
clause, of one ending with comments of its own, ends past them** (an
empty place, so nothing is mapped there, `commented_clause` in
`to_oxc/statements.rs`): oxc prints what's left of a clause before its
end, as a block's, `default:\n// ..`, as Prettier lays it out.
`test/lowering-comments-ends.test.ts`, "a last arm of only comments keeps
them in its clause"; mutation `last-clause-comments-unflushed`.


## Amendment: comments before a jump tidy takes are where it was (rd109, 2026-10-29)

React DOM's `replayUnblockedFormActions` writes comments before a
`break` and a `continue`:

```rust
if blocked_on.is_none() {
    // Not blocked: skip past it.
    i += 3;
    continue;
} else {
    // We're blocked. We'll retry later.
    break;
}
// ..
if let Some(inst) = inst {
    // Ready: delete it from the queue.
    queue.remove(i);
    log.push(inst);
    // Continue without incrementing the index.
    continue;
}
```

MIR has no statement of either jump: the loop's `break` is a `goto` of no
place to the function's `return;`, and tidy takes a `continue` the loop's
end is anyway. With no statement after them in their branch, the printer
put both comments before the innermost statement around them, the
`while`.

**A `break` or `continue` the program wrote with comments right before it
keeps them where it's written** (`sources::commented_jump`, of what a
branch does last, `structure::last_done`):

- where what the branch does last is the jump itself, as MIR places a
  `break`'s `()`, the jump MIR made of it is where it's written, `return;`
  here, and its comments before it;
- else, once tidy is done, the comments are after what's before the jump
  (`Hints::commented_jumps`, tidy's last pass, `commented jumps`), as a
  block that ends in comments has them, before its `}`, or before a jump
  tidy kept there, which they're printed before anyway.

```js
if (blockedOn == null) {
  // Not blocked: skip past it.
  i += 3;
} else {
  // We're blocked. We'll retry later.
  return;
}
// ..
if (inst != null) {
  // Ready: delete it from the queue.
  queue.splice(i, 1);
  log.push(inst);
  // Continue without incrementing the index.
} else { .. }
```

The mark is placed after tidy, not where MIR is read, as the tenth run's
design had it: a `Comments` statement before the jump while tidy reads
the body stopped its matches, `[.., If, jump]`, and put a comment in the
wrong branch of a `let … else`'s guard; tidy's passes would each have to
look past it. Only a jump with comments right before it is marked, so a
body of none reads as before: the `return;` of a `break` given its place
is an `else` where it was `if (c) { return; } rest`.

Upstream's `continue;` and `break;` there are still tidy's (`return;`,
the `else`): what's written, not where its comments go, and a decision
of its own.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Comments only.

Held by `test/lowering-comments-jumps.test.ts`, run, and the mutations
`commented-break-unplaced`, `commented-continue-unkept`
(`mir/structure.ts`) and `commented-jump-uncommented` (`sources.ts`).

## Amendment: a block that ends with comments after an `if` (2026-10-30, rd219, #428)

A block that ends with comments after an `if`/`else`, its last, gives no
`()` of its own whose span marks where they end: the `if`/`else`'s
branches give theirs. So the comment, `// TODO: ..` of React's
`startGestureTransition`, had no statement after it in the block and was
printed before the loop it's in. Now **each such block of a body's
(`sources::tail_if_comments`, of its HIR) puts its comments after its
`if`**: where the `if` is written, its keyword to its test's end, as its JS
`if` is spanned, and where they end, the block's `}`, which tidy puts after
that `if` (`commented_jumps`, rd109). A loop's body is such a block, as an
`if`'s is. Held by `lowering-comments-ends.test.ts`'s "a comment ending a
loop's body after an if/else is at its end" and the mutation
`tail-if-comments-unseen` (`lower/sources.ts`).
