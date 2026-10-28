# 0974. A `while` that steps its variable last is a `for`

Status: Accepted. Amended below: a walk, `child = child.sibling`, is a
`while` again; only a count, `i++`, `at -= 2`, is a `for` (user decision,
2026-10-28).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the `for` runs its start, test, body and step in the order the
`let` and the `while` did, under the conditions below.

## Context

React's ReactFiberPerformanceTrack walks a fiber's children so:

```js
for (let child = fiber.child; child !== null; child = child.sibling) {
  selfTime -= child.actualDuration;
}
```

Rust has no such `for`. The port writes `let mut child = fiber.child.clone();
while let Some(c) = child { ..; child = c.sibling.clone(); }`, which printed
`let child = fiber.child; while (child != null) { ..; child =
child.sibling; }`. `js::StmtKind::For` stepped only by `name++` or `name--`
(ADR 0904), so no pass could write upstream's loop. Found porting React
(pt5(c)).

## Decision

**A `let` right before a `while` whose body gives the variable its next
value of itself last is the `for` that steps it**, `for (let child =
fiber.child; child != null; child = child.sibling) { .. }`, `for (let i = 0;
i < n; i++)`, `for (let at = n; at > 0; at -= 2)`: preparation's
`prepare/stepped_whiles.rs`, after `compound` writes `x -= 2`, on the
completed tree. `js::Step` says what a `for`'s update does, `Up`, `Down`, or,
only preparation's, `To(value)`, `name = value`, and `By(op, value)`, `name
+= value` or `-=`; each walker reads its value as the `for`'s start and test.

Only where:

- **Nothing after the loop in its block names the variable**: the `for`'s
  `let` is its own, `return child != null` after it would throw.
- **The test reads it, and the body's last statement alone sets it, of
  its own value**: `child = child.sibling`, `i++`, `at -= 2`. Not a step of
  another's value, `parent = node.return`, which React writes as a `while`
  (getRootForUpdatedFiber), nor a bit's, `rest &= rest - 1`, as React's
  getEntangledLanes clears its lanes in a `while`; nor a body that sets it
  before, `if (at === 9) { at = 4; }`, a handle on it included.
- **No `continue` of the loop before the step** (`js::continues`, of its
  label too): a `for`'s `continue` runs the step that the `while`'s skips.
- **The step reads nothing the body declares**: `child = current.sibling`
  of a `const current = child;` in the body can't see it after the body's
  block. The corpus caught this as a `ReferenceError`.
- **No function in the loop names the variable**: a `for`'s `let` is a
  variable for each turn, which a closure made in a turn keeps, where the
  `while`'s is one for all. Rust's borrow checker leaves no closure made in
  the loop that reads the variable after its step, so this holds the rule
  apart from what rustc checks rather than any program compiled today.

## Alternatives

- **A `for` of any `while` before which its variable is declared**, React's
  parent walks too, `for (let node = fiber; node !== null; node =
  node.return)`: upstream writes those as `while`s, and its `for`s step the
  variable of itself. The rule follows upstream's split.
- **Keep the `while`**: correct, and upstream's `for` is lost.

## Consequences

- The corpus' and the snapshots' counting `while`s are `for`s, `for (let i
  = 0; i < n; i++)`, and the guide shows the fiber walk (Loops).
- Held by `test/corpus/while_let_steps.rs`, run against native Rust: a walk
  of siblings, one that returns, a `continue` (`ticks`), a read after,
  a step reading the body's `const`, a closure keeping a copy, a body
  setting the variable, a test of another and a step of two. Mutations in
  `scripts/mutations/prepare/stepped_whiles.ts` and `for-step-by-assigned`
  (`to_oxc/statements.ts`).

## Amendment: a walk is a `while`; only a count is a `for`

Measured on the React port (react-in-rust at eea19e3, its reconciler
built with and without the walk's rule): the rule made 60 of its `let`
and `while let` walks `for`s, `for (let child = ..; child != null; child
= child.sibling)`. Three are upstream's, ReactFiberPerformanceTrack's
logComponentRender and the two depth counts of ReactFiberTreeReflection's
getLowestCommonAncestor, `for (let tempA = nodeA; tempA; tempA =
getParent(tempA))`; the other 57 are `while`s upstream:
ReactFiberCommitViewTransitions' 17, ReactFiberCompleteWork's 5,
ReactChildFiber's, ReactFiberCommitHostEffects', ReactFiberScope's,
ReactFiberTransition's `root = root.next`, ReactFiberHooks'
resetHooksOnUnwind `hook = hook.next`, ReactFiberNewContext's `dependency
= dependency.next`, ReactFiberGestureScheduler's `prev = prev.next` and
more. React writes a walk of a list as a `while`, and a `for` of one
rarely; the counts it writes as `for (let i = 0; i < n; i++)` the rule
made as upstream (ReactFiberClassUpdateQueue, ReactFiberAsyncAction,
ReactFiberHooks).

**A walk, a step that gives the variable a value of itself, `child =
child.sibling`, `at = (at / 2) >>> 0`, stays the `while` it was** (user
decision, 2026-10-28). **A count, a step of `+=` or `-=`, `i++`, `at -=
2`, is the `for` it was made**, under the same conditions. `js::Step::To`,
the walk's update, is gone; `js::Step` is `Up`, `Down` and `By(op,
value)`. A count whose step wraps as an integer's, `i = (i + 1) >>> 0`,
is no `+=`, so it is a `while` again too, as it was before this decision.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above: either
form runs the same.

Alternatives considered: a `for` of a walk where upstream writes one,
three of sixty, which nothing in the Rust says (the program writes both
alike); a `for c in std::iter::successors(..)` as the `for`, which isn't
how a Rust programmer walks a list of `Rc`s and makes an iterator React
has none of.

Consequences: `test/corpus/while_let_steps.rs` holds walks as `while`s
and counts as `for`s, a count read after the loop (`found`), set before
its step (`jumps`) and kept by a closure (`counted`) included; the guide
shows a count and a walk (Loops). The mutation `stepped-while-of-another`
went with the walk's code.
