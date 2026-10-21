# 0536. A blanket impl over `?Sized` types keeps its trait's defaults

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0511](0511-the-crates-iterator-extension-traits.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error compiles as Rust's does.

## Context

itertools' `Itertools` is an extension of `Iterator` every iterator has,
`?Sized` ones too, `dyn Iterator` among them:

```rust
impl<T> Itertools for T where T: Iterator + ?Sized {}
```

Its dictionary names each method's instance, which rustc resolves. Of a
blanket impl over a `?Sized` type it won't, as a `dyn` might be that type
too; rust-js took the impl's own item then, but this impl has none: each
method is its trait's default, which it keeps. It was refused, "this trait
implementation".

## Decision

**In the dictionary of a blanket impl over a `?Sized` type, a method the
impl doesn't write is its trait's default**, the instance rustc gives a
`Sized` one's, as the impl keeps it.

## Why

`blanket_unsized_impl` calls such an impl's defaults, one by value and one
by `&mut self`, of a `Vec`'s iterator, a range, and a `?Sized` type
parameter, against native Rust; without this, it's refused.

## Consequences

- itertools' `Itertools` impl compiles.
- A mutation refuses the default.
- Calling such a method on a `dyn Iterator` is still refused, "this trait
  object supertrait": a `dyn`'s dictionary is its own trait's and its
  supertraits', and a blanket impl's over the `dyn` isn't made of it yet.
