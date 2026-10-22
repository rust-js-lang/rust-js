# 0589. A `Drop` dropped in one place is its body there

Status: Accepted. Amended by [0608](0608-a-drop-is-its-body-at-each-drop.md):
a `Drop` is its body at each drop, a written guard's fields `let`s, a
one-field tuple's its field. Extends [0098](0098-destructors.md)'s destructors and
[0581](0581-a-closure-called-as-a-statement-is-its-body.md)'s bodies in
place.

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
