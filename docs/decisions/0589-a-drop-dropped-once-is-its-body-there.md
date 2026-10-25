# 0589. A `Drop` dropped in one place is its body there

Status: Accepted. Amended by [0608](0608-a-drop-is-its-body-at-each-drop.md):
a `Drop` is its body at each drop, a written guard's fields `let`s, a
one-field tuple's its field. Extends [0098](0098-destructors.md)'s destructors and
[0581](0581-a-closure-called-as-a-statement-is-its-body.md)'s bodies in
place.
Amended by [0705](0705-a-guards-scope-reads-as-any-body.md): a guard's
field of a variable nothing sets later is that variable.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Each rewrite runs
the same code in the same order.

## Context

React's scheduler restores its state however `flushWork` is left; in
Rust, a guard's `Drop` does:

```rust
let _restore = Restore { previous_priority_level: CURRENT_PRIORITY_LEVEL.get() };
work_loop(initial_time)
```

```js
const _restore = {previousPriorityLevel: currentPriorityLevel};
let result;
try {
  result = workLoop(initialTime);
} finally {
  restoreDrop_drop(_restore);
}
return result;
```

Upstream's is its `try`'s `finally`, its state in a `const`:

```js
const previousPriorityLevel = currentPriorityLevel;
try {
  return workLoop(initialTime);
} finally {
  currentPriorityLevel = previousPriorityLevel;
}
```

## Decision

**A `Drop` impl's `drop` that's dropped in one place, in its own module
and called by no other, is its body there** (`drops::inlined_drops`), as a
closure called where it's made is (ADR 0581, `js::applied_call`), and the
function is gone. One dropped in two places stays a function each drop
calls.

**The guard it's given, made there of its fields and read only by them,
is its fields**: `const previousPriorityLevel = currentPriorityLevel;`,
where no other name of the function is one. One read whole, given to a
function, stays an object.

**`let r; try { ..; r = e; } finally { .. } return r;` is `try { ..;
return e; } finally { .. }`** (`unwinds::returned_through_finally`), which
makes `e` and then runs the `finally`, as before; of any variable so
given, where nothing else names it.

## Why

- **It's how JS is written**: a `finally` that restores what it changed,
  not a function a guard's type is named after.
- **It's what Rust means**: the guard exists to run its drop where it's
  dropped; with one drop, that's one place.

## Alternatives

- **Inline every function called once**: a function the program names
  is the program's, and its name reads. Rejected; a `Drop`'s function is
  rust-js's.

## Consequences

- Thirty corpus cases' drops are their bodies; a `try` that gives what a
  function returns returns it. The corpus is 105 lines shorter, so the
  shares of `while (true)`, `$iter` and `return;` grow by a tenth in
  10,000 lines, their counts the same; renames fall from 839.4 to 838.7.

## What holds it

- `test/lowering.test.ts`, "a Drop dropped once is its body where it's
  dropped": a guard of a thread-local restored by its field, through a
  panic; one dropped twice, a function; one read whole, an object; one
  dropped in two modules, a function both call.
- Mutations: a twice-dropped one inlined, the guard kept whole, one read
  whole taken apart, any function inlined, one called elsewhere inlined,
  the `try` not returning, the call never applied.

## Amended (2026-10-24)

The drops are inlined in an order, not a set's. A set's order changed
from one build to the next, and with it the JS: `Holder`'s drop, which
calls `Md`'s, inlined after `Md`'s, left its guard's fields `[3]`, read
`inner[0]`; before it, taken apart, `3`. Now each in turn is one no other
drop left calls, the caller first, so each guard is taken apart where
it's made, and the module's order among those. Held by the mutation
`drops-inlined-callees-first`, whose JS
`test/corpus/refused_drops_and_displays.js` isn't.

## Amendment: not a drop that calls itself

A `Drop` whose body drops a value of its own type, `let old =
mem::replace(self, ..)` dropped at its end, calls itself. Written at each
of its calls, its body made another call to write, and the compiler never
finished: rustc's `drop/issue-23611-enum-swap-in-drop.rs` timed out
(#229). A drop whose own body calls it stays a function, called, as
Rust's drop glue runs it again. `test/corpus/drop_of_its_own_type.rs`
holds it, with the mutation `drop-of-its-own-type-inlined`.
