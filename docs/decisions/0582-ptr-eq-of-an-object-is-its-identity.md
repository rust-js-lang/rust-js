# 0582. `Rc::ptr_eq` of an object is its identity, uncounted

Status: Accepted. Amends [0320](0320-counted-rcs.md), which counted every
`Rc` that `ptr_eq` asks of. Amended: not of a `#[repr(transparent)]`
struct, which is its field (ADR 0584), a number's `===` its value.
Amended by [0863](0863-a-refcell-of-an-object-is-the-object.md): of an
`Rc<RefCell<T>>` of an object too.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Two `Rc`s point at
one value exactly when their JS objects are one.

## Context

React's scheduler asks whether the task it ran is still the first:

```rust
if peek(task_queue).is_some_and(|first| Rc::ptr_eq(first, &current_task)) {
    pop(task_queue);
}
```

ADR 0320 counted an `Rc` that `ptr_eq` compares, `{ value, strong, weak }`,
as an uncounted one is the value it points at, and two `Rc::new(1)`s are
`1 === 1`. A counted `Rc` has a destructor, which made the scheduler's
`with_borrow_mut` closure that moves a task refused, "of a value with a
destructor".

## Decision

**`ptr_eq` of an `Rc` of a struct of the crate's with fields, a JS object
whose `===` is its identity, counts nothing** (`reads_rc_count`): it's
`a === b` of the objects. Of anything else, a number, a string, a unit
struct, it counts as before.

(Amended: nor of a `#[repr(transparent)]` struct, which ADR 0584 made its
field, `Meters(1.0)` the number `1`: two `Rc`s of equal ones were `1 ===
1`, `true`, where Rust's point at two allocations. Found writing ADR 0844;
it counts as a number's does.)

## Why

- **It's what JS compares**: an object made by `Rc::new` is one object,
  as the `Rc`s of it share one allocation.
- **What counting is for**: an answer only counts give. Identity of an
  object isn't one.

## Alternatives

- **Count each `Rc` it compares**, ADR 0320's: a destructor for every
  task, and its thread-local's closure refused. Rejected.

## What holds it

- `test/lowering-cells.test.ts`, "ptr_eq of an Rc of a struct is === of its
  object, uncounted": a task's `a === b`, a number's and a transparent
  struct's still counted, run.
- Mutations: an object's counted, a number's not, a transparent struct's
  not (`scripts/mutations/lower/recognition/wrappers.ts`,
  `ptr-eq-of-transparent-uncounted`).
