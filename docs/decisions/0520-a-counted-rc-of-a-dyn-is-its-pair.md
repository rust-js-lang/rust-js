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
[counted-`Rc`s-across-crates draft](0523-libraries-share-one-counted-rc.md)'s
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

## Amendment: an upcast of a counted `Rc<dyn Sub>` shares its count (#186)

A counted `Rc<dyn Animal>` made an `Rc<dyn Named>`, `let n: Rc<dyn Named>
= a.clone();`, was refused: the pair is one object of the `Rc`'s count and
its trait's dictionary, and the upcast needs another dictionary but the
same count, which every clone shares. **It's `$rcUpcast(rc, "Named",
rc.impl.Named())`, a pair of the supertrait's dictionary whose value and
counts are the `Rc`'s own, read and written through to it**, so a clone,
a drop, `strong_count`, a `Weak` and `try_unwrap` through either are the
one allocation's, as Rust's upcast shares it. One for each supertrait,
kept on the `Rc`, so two upcasts of it are the same box, as `Rc::ptr_eq`
finds them. Held by `test/corpus/rc_dyn_upcast_counted.rs` against native
Rust, two levels of supertraits too, and the mutations
`counted-rc-upcast-refused` and `rc-upcast-view-uncached`.

## Amendment: a counted `Rc` of a `Box<dyn T>`'s pair (#399)

`Rc::from(b)` of a `Box<dyn T>`, `b.into()` and `Arc::from(b)` made a
counted `Rc<dyn T>` of the box held whole: `{ value: b, strong: 1, weak: 0
}`. Its calls read `r.impl`, as a counted `dyn`'s pair is the `Rc` itself,
and threw `TypeError`, where Rust runs. A wrong answer, so ADR 0262's
"results change" case: Rust's behavior is kept.

`counted_same` now makes a counted `Rc` of a pair `dyn` of the pair's own
fields, with the counts: `{ ...b, strong: 1, weak: 0 }`, or the fields
themselves where the pair is made there. That's what `Rc::new(x)` unsized
to `Rc<dyn T>` is, so clones, counts and the drop read it the same way.

- `test/corpus/counted_rc_of_boxed_dyn.rs` checks `Rc::from`, `into()`
  and `Arc::from`, a clone, and a `Drop` run through the `dyn`, against
  native Rust.
- The mutation is `counted-rc-of-boxed-dyn-nested`.
