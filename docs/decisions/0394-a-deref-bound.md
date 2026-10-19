# 0394. A `Deref` bound is given its dictionary, `{ deref }`

Status: Accepted. Extends [0108](0108-generic-operators-and-into.md) and
[0023](0023-strings-references-shared-state.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what `deref` gives is
the value Rust's gives, and of std's, the same JS value.

## Context

Generic code reaches through a pointer of whatever kind it's given:

```rust
fn shout<P: Deref<Target = str>>(p: P) -> String {
    p.to_uppercase()
}
impl<P: Deref<Target = T>, T> Deref for Pin<P> {
    fn deref(&self) -> &T { &*self.0 }
}
```

`<P as Deref>::deref` was refused: `Deref` had no dictionary. Two of
rustc's tests failed on it. A `&mut ()` kept in one, which has no place in
JS, was refused too.

## Decision

**A `P: Deref` is given a dictionary, as ADR 0108's operators and `Into`
are, of `deref`: std's, the value itself, `(x) => x`, where its target is
the same JS value, a reference, a `Box`, an `Rc` or text (ADR 0023); the
crate's own, its impl's `deref`.**

```js
function shout(p, PDeref) {
  return PDeref.deref(p).toUpperCase();
}
shout("box", { deref: (x) => x });
```

- **Where the pointer's type is known, nothing changes**: the crate's
  `deref` is called as it is, std's, a guard's value or a `Cow`'s text, is
  recognized in place as before, as a known writer's is (ADR 0392); only
  generic code reads a dictionary.
- **A crate's `Deref` impl's dictionary is written only where it's
  reached**, as a derived `Debug`'s is: every newtype has one, and generic
  code reads few.
- **A `Box`'s `deref`, and a shared reference's, are std's `Same`**, as a
  `String`'s and an `Rc`'s were: MIR never called them, a dictionary does.
- **A `&mut ()` is nothing**, `undefined`, as writing a `()` is.

## Why

- **It's the value**, as Rust's `deref` gives it.
- **It's tested**: the corpus's `generic_deref.rs` derefs a `Box<str>`, a
  `String`, an `Rc<String>`, the crate's own type and a `&i32` in generic
  code, and keeps a `&mut ()`; rustc's
  `associated-types/associated-types-conditional-dispatch.rs` and
  `methods/inherent-method-resolution-on-deref-type-53843.rs` pass.

## Consequences

- A `P: DerefMut` in generic code is still an error. (Amended below: it's
  given a dictionary too.)

## Amendment: `DerefMut`

A `P: DerefMut` in generic code, `p.push(1)` of a `P: DerefMut<Target =
Vec<u32>>`, or either's `&mut *inner` of an `L: DerefMut`, was refused.
It's given a dictionary as `Deref` is, `{ Deref, deref_mut }`. Case N
([0262](0262-when-rust-and-js-disagree.md)): the same value, changed in
place.

```js
function push_one(p, PDerefMut) {
  PDerefMut.deref_mut({ get value() { return p; }, set value(value) { p = value; } }).value.push(1);
  return p;
}

push_one([8], { Deref: () => ({ deref: (x) => x }), deref_mut: (cell) => cell });
```

- **Given a cell, giving a cell**: `deref_mut` takes `&mut self` and
  gives `&mut Self::Target`, each a box in generic code (ADR 0454).
  std's, of a pointer whose target is the same JS value, a `Box`, text,
  a `Vec` or a JS object, is the cell it's given; of a `&mut` or a guard,
  which holds a cell of its target, what's in it, `cell.value`. Others
  are errors.
- **It holds its `Deref`**, as a dictionary of `Error` holds its
  `Display` (ADR 0141): `p.len()` of a `P: DerefMut` reads it. Whether a
  dictionary holds its supertraits' is `holds_supertraits`; their
  entries, everywhere a dictionary is made, `supertrait_entries`, which
  had been written out four times, and an associated type's bounds',
  `item_bound_entries`, twice.
- **The crate's is its impl's**, written only where generic code reads
  it, and std's of a known type is written in place, a guard's `*g += 1`,
  as `Deref`'s is.
- The `generic_deref_mut` corpus case pushes to and bumps through a
  crate's `DerefMut`, a `Box`, a `&mut`, a guard and an either-like
  enum of two, against native Rust.
