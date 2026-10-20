# 0520. A counted `Rc<dyn T>` is its pair

Status: Accepted. Amends [0320](0320-counted-rcs.md); extends
[0331](0331-dyn-any.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same counts,
identity and values; only how JS holds them changes.

## Context

A counted `Rc<dyn T>` was `{ value: { value: x, impl }, strong, weak }`:
the `Rc` around the `dyn`'s pair. `Rc::downcast` of one gives an
`Rc<T>` of the same allocation, whose counts its `Rc<dyn T>` clones share;
an `Rc<T>`'s `value` is `x`, its `dyn` clones' was the pair. No object
could be both, so the downcast was refused, as were a `dyn Any`'s
`downcast_mut` and an `Rc<dyn Any>`'s `downcast` where the `dyn` might
hold a value with a destructor. These were two of the four gaps the
[counted-`Rc`s-across-crates draft](draft/counted-rcs-across-crates.md)'s
trial found.

## Decision

**A counted `Rc<dyn T>` is its pair: `{ value: x, impl, strong, weak }`.**
The one object is both the `Rc` and the `dyn`'s `{ value, impl }`, so an
`Rc<T>` of the same allocation is the same object, its `value` `x`.

- **One answer to what a counted `Rc` points at as JS reads it**:
  `counted_pointee`, its `value`, or of a `dyn`'s the `Rc` itself. `*rc`,
  `==`, the order, the hash, `{}`, JSON and what's shown through it ask it.
- **Its last drop gives the pair**: `$rcDrop` reads it off the `Rc`, which
  has an `impl` only as a `dyn`'s.
- **A downcast is the same `Rc`**: `Ok(rc)`, or `Err(rc)` given back.
- **A `dyn Any`'s downcasts drop nothing**: `is`, `downcast_ref`,
  `downcast_mut` and `downcast` lend or move what they're given.

An `Rc` made elsewhere and only then made a `dyn` stays refused: its other
owners hold it as what it is (ADR 0320).

## Why

- **It's the same program**: `dyn_any_counted_rc` downcasts a counted
  `Rc<dyn Any>` that fails and one that holds a value with a destructor,
  and drops each clone, against native Rust.
- **It reads better**: `b.impl.area(b.value)`, not
  `b.value.impl.area(b.value.value)`.

## Consequences

- `rc_dyn_destructors` and the corpus's other counted `dyn`s lose a level.
- `dyn_any_counted_rc`, a `compile-fail` case, runs; it's still refused as
  a library, until the draft is accepted.
- Mutations read the `dyn`'s `value`, take the downcast for what it points
  at, give the last drop the value, and refuse the downcasts.
