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

- A `P: DerefMut` in generic code is still an error.
