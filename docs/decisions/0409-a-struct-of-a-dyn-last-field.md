# 0409. A struct of a `dyn` last field is the struct, with its `impl`

Status: Accepted. Generalizes [0397](0397-a-refcell-of-a-dyn.md), extends
[0049](0049-traits-and-generics.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each call through the
field is the impl Rust's would call, and what's changed through the
reference is the struct's. Where that can't be kept, D: a compile error.

## Context

A struct whose last field is unsized, `struct Fat<T: ?Sized> { f1: isize,
ptr: T }`, can be made a `&Fat<dyn ToBar>` of a `&Fat<Bar>`. rust-js
refused it: the field would have to be a `dyn`'s pair, `{ value, impl }`,
which it isn't. ADR 0397 had already answered the same question for one
such struct, std's `RefCell<C>` made a `RefCell<dyn T>`: the same cell,
given `C`'s `impl` in place.

## Decision

**A `&Fat<C>` made a `&Fat<dyn T>` is the same struct; the struct the
`dyn` field is in is given `C`'s `T` as its `impl`, in place, as a
`RefCell`'s cell is. A reference to the field is its pair: its value and
that `impl`, `{ value: x.ptr, impl: x.impl }`, or of a `&mut`, a pair on
the field's place. A call through it is the `impl`'s.**

```rust
fn foo(x: &Fat<dyn ToBar>) -> isize { x.f1 + x.ptr.to_val() }
foo(&f1);
```

```js
function foo(x) {
  return (x.f1 + x.impl.to_val(x.ptr)) | 0;
}
foo($dynCell(f1, bar1ToBar()));
```

- **One step gives an `impl`** (`given_impl`), a `RefCell`'s and a
  struct's alike: `$dynCell`, not enumerable, so what compares, copies
  and shows a struct doesn't see it; of a value made a `dyn` of two
  traits, both's methods, refused where they share a name.
- **The struct the field is in** is given it: of a `&Fat<Fat<dyn T>>`,
  the inner one, so a `&Fat<dyn T>` of it, `&x.ptr`, has it too.
- **A pair made of parts that read the same** is its parts where it's
  called: `x.impl.to_val(x.ptr)`, not a `const` of the pair.

## Why

- **It's the program Rust runs**: the `dyn_struct_tails` corpus case calls
  through the field of a struct, a nested one's, a `ref` binding's, and a
  `&mut self` method that changes a struct's field and a number field,
  each with native Rust.
- Three of rustc's tests pass: `dst-trait.rs`, `match-dst-struct.rs`,
  `multiple-drop-safe-code-25549.rs`.

## Consequences

- **Behind a reference only**: a `Box<Fat<dyn T>>` or an `Rc` of one,
  which would drop its last field through its `impl`, is still refused,
  as is a `dyn Debug` last field, which would be the string it shows (ADR
  0060), and a `&Fat<dyn Sub>` made a `&Fat<dyn Super>`.

## Amendment: a `Box` of one

A `Box<Fat<dyn T>>` is the struct given its `impl`, as a reference's is,
and drops its last field through it, `fat.impl.$drop?.(fat.inner)`, where
Rust drops it; so is one of a `dyn` of std's or of no trait (ADR 0436).
An `Rc` of one, which may be counted, is still refused.
