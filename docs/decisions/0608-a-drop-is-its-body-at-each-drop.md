# 0608. A `Drop` is its body at each drop

Status: Accepted; amended (a name its body shares with the caller is
named apart). Amends [0589](0589-a-drop-dropped-once-is-its-body-there.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: each drop runs the
same code where it ran.

## Context

ADR 0589 made a `Drop` dropped in one place its body there, and kept one
dropped in several a function each drop calls. React's scheduler restores
its priority three times, `unstable_runWithPriority`, `unstable_next` and
in `unstable_wrapCallback`'s closure, and upstream writes each:

```js
var previousPriorityLevel = currentPriorityLevel;
currentPriorityLevel = priorityLevel;
try {
  return eventHandler();
} finally {
  currentPriorityLevel = previousPriorityLevel;
}
```

where the port had `const _restore = [currentPriorityLevel]; .. finally {
restorePriorityDrop_drop(_restore); }`: a function named by rust-js, not by
the program, and a guard boxed in an array. Its `performWorkUntilDeadline`
kept `const more = { hasMoreWork: true }`, a field written, where upstream
has `let hasMoreWork = true`.

## Decision

**A `Drop` is its body at each drop in its module, a closure's too**,
where each is a call as a statement and its body fits there; where one
doesn't, a name of its body's the caller's own, each calls the function,
as before.

**The guard is its fields**, as ADR 0589 has it, and:

- **a field written through it is a `let`**: `let hasMoreWork = true;
  try { hasMoreWork = flushWork(currentTime); } finally { if (hasMoreWork)
  .. }`;
- **a one-field tuple's guard is its field**, by the guard's name, read
  now, so without Rust's `_` for what's only dropped: `let _restore =
  Restore(LEVEL.get());` is `const restore = level;`.

## Why

- **A destructor is what runs where the scope ends**; JS writes that code
  in the `finally`, at each, as upstream does. Its function's name is
  rust-js's, `restorePriorityDrop_drop`, which no JS developer writes.

## Consequences

- A larger `Drop` dropped in many places is written at each: the corpus
  is 87 lines shorter all the same, as one-line drops are most.
- A guard whose field another guard's took, `name` of two `Loud`s, stays
  an object.

## What holds it

- `test/lowering.test.ts`, "a Drop dropped once is its body where it's
  dropped": two drops, one in a closure, a written guard, a one-field
  tuple's, and one whose body can't fit, which stays the function.
- Mutations: the drops kept as calls, inlined where one can't be, a
  closure's skipped, the guards kept whole.

## Amendment: not a word JS keeps

The value kept to be dropped is read by its name without the `_` where
that's free, `_restore` as `restore`, but `_in` became `in`, `_new`
`new`, and `_eval`, `_arguments`, `_undefined` names strict mode refuses:
the module couldn't be parsed, or failed as it loaded (#127). The `_` is
taken off only where what's left isn't a word JS keeps,
`names::kept_word`, the one list of them. Held by
`test/corpus/drop_guard_kept_words.rs` and the mutation
`drop-guard-kept-word`.

## Amended (2026-10-25): a name its body shares with the caller is named apart

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same code run
in the same order.

React's `startTransition` restores what it changed in a `finally`; its
port's `FinishTransition` guard, whose `Drop` does it, stayed
`finishTransitionDrop_drop(_finish)`, its guard `const _finish = { ..
}`: its body's `borrow`, a `RefCell`'s, was the name of the caller's own
`borrow`, in an `if` before it, and a drop whose body declares a name of
the function's stayed the function each drop calls.

**What a drop's body declares at its top that's another name of the
function, or the variable it's given, is named apart there, `borrow$1`**,
as lowering names a closure's body's apart from its function's (ADR
0352), and **reclaimed where JS's scopes let it be** (ADR 0357): `borrow`
in the `finally`, where the caller's is another block's. One its scope
can't let be, a parameter's `doubled`, stays `doubled$1`, where the drop
was a function a person never writes. The same is so of a closure called
where it's made, `js::applied_blocks`'s, which shares the rule
(`js::applied_call`).

Held by `test/lowering.test.ts`, "a Drop dropped once is its body where
it's dropped" (`Twice`'s `doubled$1`, `Shown`'s `bumped` reclaimed) and
"a guard dropped around catch_unwind is its try's catch and finally"; the
mutations `applied-clash-refused` and `applied-clash-kept`
(`js/applied.ts`).
