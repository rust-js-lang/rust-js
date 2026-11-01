# 1047. An index is bound by each length it's tested below, in a closure too

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md),
whose integer had one bound by a slice's length, and
[0577](0577-a-function-only-calls-reach.md), whose functions only calls
reach are told what each call gives them: a closure is told what's known
where it's made.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Nothing a program
does changes: an index left unchecked is one Rust's check would pass.

## Context

React's areHookInputsEqual compares two hooks' dependencies item by item:

```rust
let mut i = 0;
while i < prev_deps.len() && i < next_deps.len() {
    if OBJECT_IS.with(|is| is(&next_deps[i], &prev_deps[i])) {
        i += 1;
        continue;
    }
    return false;
}
```

Its JS checked both indexes, `is($index(nextDeps, i), $index(prevDeps,
i))`, where upstream reads `nextDeps[i]` and `prevDeps[i]`. Two things
lost what the test shows:

- An integer's range held one bound by a length (ADR 0576): `i <
  next.len()` put `i` below `next`'s length in place of `prev`'s, so
  `prev[i]` was checked even with no closure.
- A closure's body was walked knowing nothing, as its own body: what the
  loop's test shows of the `i` it captures, it never knew.

## Decision

**An integer's range holds a bound by each slice's length, and by each
integer's value, it's known below** (`known::Bounds`, a set, one bound of
a slice): a test narrows by each side's bounds besides its own; a join
keeps each slice both are bound by; an operator's result is bound by each
of its operands'; an index is in bounds where any of its bounds is the
slice's.

**A closure the body makes is told what's known where it's made, of what
it captures that nothing changes while it lives** (`ranges/closures.rs`):
a variable it captures by shared reference, which Rust's borrows keep
unwritten while the closure lives, and one it takes by value and never
writes, as rustc's capture says; of each, its range and its bounds by
such slices, and each such slice's least length and what it's as long as.
Not a slice in a `RefCell`, which any code may change (ADR 0881); nor a
`bool`'s test, a cell still empty or what reaches cells, which are of the
body's own code. Made more than once, a loop's, it's told what each
place holds, joined; made where the walk lost its way, nothing. Its body
is walked from what it's told, as a function only calls reach is (ADR
0577), again each time it's told otherwise.

areHookInputsEqual reads `is(nextDeps[i], prevDeps[i])`, as upstream.

## Why

- **What Rust's borrows prove holds in the closure**: a shared borrow
  keeps what it captures as it was for as long as the closure can run,
  so each call starts from what was known where it was made.
- **Each bound is true**: an `i` tested below two lengths is below each;
  keeping one dropped what the program's test proved.

## What holds it

- The corpus case `index_bounds_closures`, against native Rust: both
  bounds without a closure and in one, a nested closure, an index of a
  slice the test didn't bound, which panics as Rust's does, a `move`
  closure that changes what it took, and a `RefCell`'s slice a closure
  clears.
- `lowering-index-loops`'s "an index below two lengths reads each in
  bounds, in a closure too": areHookInputsEqual's loop, run.
- Mutations `join-one-bound` (`ranges/known.ts`), `value-bound-over-length`
  (`ranges/narrowing.ts`, now of a bound kept beside the other),
  `closure-told-nothing` (`ranges.ts`), `closure-told-what-it-changes`,
  `closure-told-through-a-mutable-borrow` and `closure-told-a-cell`
  (`ranges/closures.ts`). A closure is told before its captures are walked,
  so `closures::told` alone says what it's told: a `&mut` capture's borrow
  forgets the variable after. Not held by a test: a closure made where the
  walk lost its way is told nothing, which only a `#[loop_match]`'s or
  assembly's jump does.
- 19 mutations of 0576's, 0632's, 0701's, 0709's and #362's to #370's
  rules, rewritten for the bounds' set and all caught as before.
