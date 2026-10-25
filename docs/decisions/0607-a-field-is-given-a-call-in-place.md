# 0607. A variable's field is given a call's value in place

Status: Accepted. Amended by [0879](0879-an-if-and-a-closures-captures-read-as-written.md): a place JS reaches as a variable, a closure's capture's `*(_1.0)`, is given the value in place too, `result = callback();`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same value is
made, then set, on the same object.

## Context

Rust makes the value before it reaches the place it's assigned to; JS
reaches the place first. So rust-js made a value with effects first, in
a temporary: `more.has = work();` was

```js
const value = work();
more.has = value;
```

where React's scheduler has `hasMoreWork = flushWork(currentTime);`.

## Decision

**A place of a variable's fields is given the value in place**: the
object JS reaches first is the one Rust sets, since what makes the value
can't make the variable another; a closure's `more = More { .. }` assigns
the object in place (`$assign`). A place through an index or a reference,
whose index or target the value may change, keeps the temporary.

## What holds it

- `test/lowering.test.ts`, "a field is given a call's value in place",
  with a closure that reassigns the variable in what makes the value.
- Mutation: the temporary kept.
