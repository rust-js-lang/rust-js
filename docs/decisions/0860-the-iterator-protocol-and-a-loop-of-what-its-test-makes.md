# 0860. JS's iterator protocol is `js::Iterator`, and a loop is of what its test makes

Status: Accepted. Extends [0102](0102-js-and-webapi.md)'s js crate with
TypeScript's `Iterator<T>` and `IteratorResult<T>`, and
[0310](0310-any-value-as-js-reads-it.md)'s `number::to_string` with its
radix. Amended: a loop's test of a copy of a variable is the variable's (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each binding is the
call TypeScript types; the loop runs the same statements in the same
order, only written as JS writes it.

## Context

React's `mapIntoArray` (ReactChildren.js) walks what an iterable's
`@@iterator` gives by its `next()`, and keys each child by its index in
base 36:

```js
const iterator = iteratorFn.call(iterableChildren);
let step;
let ii = 0;
while (!(step = iterator.next()).done) {
  child = step.value;
  nextName = nextNamePrefix + getElementKey(child, ii++);
  ..
}
..
return index.toString(36);
```

The iterator may be a program's own, `{ next() { .. } }`, with no
`[Symbol.iterator]`, as React's tests make one: `for..of` would throw, so
it's stepped by `next()`. The js crate had no type of the protocol, and
`number::to_string(x)` no radix. And the loop a port writes of it, `loop
{ let step = iterator.next(); if step.done == Some(true) { break; } .. }`,
printed `while (true) { const step = iterator.next(); if (step.done) {
break; } .. }`, as each `while let Some(x) = stack.pop()` printed `while
(true) { const x = stack.pop(); if (x == null) { break; } .. }`.

## Decision

**`js::Iterator<T>` is TypeScript's `Iterator<T>`, its `next()` an
`IteratorResult<T>`, a struct of TypeScript's `done?: boolean`, an
`Option<bool>`, and its `value: T`.** `number::to_string(x, radix)` is
`x.toString(radix)`, `radix` TypeScript's `radix?: number`, `None` left
out.

```rust
let iterator: &js::Iterator<Option<&'static Unknown>> = ..;
loop {
    let step = iterator.next();
    if step.done == Some(true) {
        break;
    }
    let child = step.value;
    let key = js::number::to_string(f64::from(ii), Some(36));
    ..
}
```

```js
let step;
while (!(step = iterator.next()).done) {
  const child = step.value;
  const key = ii.toString(36);
  ..
}
```

- **A step's `done` is left out until the end**, `IteratorYieldResult`'s
  `done?: false`: an `Option`, whose test, `step.done == Some(true)`, is
  JS's, `if (step.done)`, as the protocol reads it.
- **The end's `value` is its return value**, TypeScript's `TReturn`, any
  value: read a step's only before its end.
- **Only the protocol**: what's iterable is a `Vec` or an iterator's
  adapters (ADR 0025); a program steps this one by `next()`.

**A loop whose first statement makes what the test that leaves it then
reads, once and first, is the loop of that test, which makes it**: `while
(true) { const x = e; if (t(x)) { break; } .. }` is `let x; while (!t(x =
e)) { .. }`, as JS writes a walk of an iterator's steps or of a regular
expression's matches. Preparation makes it, before its locals are named,
so the `let` is one name of the block it's in.

- **Not where a closure in the loop keeps it**: a `const` is a variable of
  each turn, a `let` before the loop one of all of them.
- **Not where the test reads it again**, `last == null || last.TAG !==
  "Op"`, which would make it in its first read, nor where it reads
  anything first, `stack.length === 0 || top == null`, which would then
  run before what makes it.
- **`ExprKind::Assigned`, `(name = value)`**, is the JS tree's assignment
  as a value: only preparation makes one, so lowering and tidy never see
  it.

## Why

- **It's React's JS**: `while (!(step = iterator.next()).done)` and
  `index.toString(36)`.
- **It's TypeScript's types**: the protocol's `Iterator<T>`, whose
  `next()`'s `done` may be left out, and `toString(radix?)`.
- **The same statements in the same order**: `e` is made, then tested,
  each turn, as before; a `continue` runs the test again, as it ran the
  loop's first statement.

## Alternatives

- **`js::Iterator<T>` a Rust `Iterator`**, stepped by `for child in
  iterator`: its `next()` is `Option<T>`, whose `Some(None)` of an
  `Option<&Unknown>` item is a box (ADR 0051), and an adapter of it would
  need JS's iterator helpers, which a program's own `{ next() }` hasn't.
- **`done: bool`**: a step that leaves it out would be a `bool` that's
  `undefined`.
- **The loop as it was**: `while (true)` and a `break` where JS writes the
  test once.

## Consequences

- Every such loop is the test's: `while let Some(t) = stack.pop()` is
  `let t; while ((t = stack.pop()) != null) { .. }`, an own iterator's
  `for` is `let n; while ((n = countdownIterator_next(iter)) != null) {
  .. }`, and the guide's Iterators section says so. A comment before the
  loop is before its `let`. A `let` whose name a variable around it has,
  a parameter's, keeps its `$1`, `let value$1;`, where the loop's `const`
  could shadow it.
- `number::to_string(x)` is `number::to_string(x, None)`.
- The readability baseline's `while (true)` count is 96, from 159; its
  `renamed x$1` count up by 7, each such `let` (ADR 0565).

## What holds it

- `test/js-builtins.test.ts`, "an iterator is stepped by its next, and a
  number written in a radix": a program's own iterator, its steps' `done`
  left out, and a `Map`'s and an array's, each keyed in base 36.
- `test/corpus/loop_assigned_tests.rs`: a `while let` of a `pop`, an own
  iterator's `for`, a closure that keeps each turn's, a test that reads
  it twice, and one that reads the stack first, run beside native Rust.
- Mutations `assigned-test-unmade`, `assigned-test-captured`,
  `assigned-test-read-first-unasked`, `assigned-test-read-twice`
  (`scripts/mutations/prepare.ts`), `reclaim-assigned-unrenamed`
  (`prepare/reclaim.ts`) and `assigned-value-alone`
  (`to_oxc/expressions.ts`).

## Amendment: a loop's test of a copy of a variable is the variable's (h4, 2026-10-27)

A `while let` of a thread-local, React's popToNextHostParent's `while let
Some(parent) = HYDRATION_PARENT_FIBER.with_borrow(Clone::clone)`, reached
tidy's loop tests as `while (true) { const hydrationParentFiber$1 =
hydrationParentFiber; if (hydrationParentFiber$1 == null) { return; } ..
}`: the `if` that ends the loop wasn't the body's first, so the loop kept
`while (true)`, and ADR 0277 couldn't read the copy in place till after,
in preparation.

**A body that copies a variable first and tests the copy tests the
variable itself, the copy after it** (`tidy/loops.rs`,
`copy_tested_first`), `if (hydrationParentFiber == null) { return; }
const hydrationParentFiber$1 = hydrationParentFiber;`, so the loop's test
is its own, `while (hydrationParentFiber != null)`, and preparation reads
the copy in place where ADR 0277 says it may, `switch
(hydrationParentFiber.tag)`. Where the test reads only variables, so
nothing runs between it and the copy, which reads the variable as the
test did; where the variable is one, or a property of one, a
thread-local's cell's, not an array's item, whose loop is this
decision's `while ((x = xs[i]) != null)`; and where what leaves the loop
reads no copy, which the loop's block declares. Where the loop's test
can't be made of it, the body is as it was.

- Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same value
  tested.
- Held by the corpus case `loop_over_a_thread_local.rs` and the mutations
  `while-of-a-copy-untested` and `while-of-a-copy-of-an-item`
  (`scripts/mutations/lower/mir/tidy/loops.ts`).
- `while (hydrationParentFiber)`, upstream's test by truth, would need an
  `Option<Rc<T>>` to be never falsy, as an `Option<&T>` is (ADR 0298):
  that changes every such test of the ports, written up for the user.
