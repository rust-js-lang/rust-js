# 0454. A `&mut` given as a type parameter's value is a box

Status: Accepted. Extends [0099](0099-mut-references.md) and
[0180](0180-generic-writers.md). Decided with the user.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same writes, to
the same place; and a compile error where generic code would have a box
and its caller the object (below).

## Context

A `&mut` to an object is the object in JS, and one to a number or text a
cell, `{ value }`. Generic code doesn't know which it has, so a generic
`&mut T` is given a box, copied back after the call (ADR 0099). But a
`&mut` given as a type parameter's value, `D` of `f(&mut v)`, was the
object, and an impl for `&mut T`, which has its `Self` in a box, read the
object's `value`: `impl<T: Get> Get for &mut T` called through `f<D:
Get>(&mut v)` threw. std's `impl Write for &mut W` and stable_deref_trait's
`StableDeref for &mut T`, which writeable and url take, were refused for
the same reason: generic code had no one form of a `&mut`.

## Decision

**A `&mut` given for a type parameter is a box, as one given for a
`&mut T` is: copied back after the call, and what's in it taken out where
the call gives it back. An impl for `&mut T` reads through it, and so do
std's `Deref` of a `&mut`, `(x) => x.value`, and its `fmt::Write`,
`$mutWriter(writer)`, the writer's own dictionary through the box.**

```rust
fn totalled<D: Total>(items: D) -> u32 {
    items.total()
}
println!("{}", totalled(&mut items));
```

```js
const items$1 = { value: items };
const value = totalled(items$1, refMutTTotal(vecTotal()));
$assign(items, items$1.value);
```

1. **One question, `mut_refs::boxed_in_call`**: of a type in a function's
   signature, through shared references, a `&mut` to its type parameter
   or its type parameter given a `&mut` to an object is a box. A
   parameter is given one, a result has its `value` taken out, and an
   impl for `&mut T`'s `&self`, called where its type is known, is given
   a box of the object, which nothing writes through.
2. **Elsewhere, refused**: a type parameter given a `&mut` to an object
   inside what generic code takes or gives, an `Option<D>`, a field, or a
   closure's parameter through `F: Fn(D)`, where generic code would have
   the box and its caller the object, as a nested `&mut T` was (ADR
   0099). A bound's own type, `D: Total`'s `D`, isn't a value.
3. **One writer through a box**, `$mutWriter`, of any writer: a `String`'s
   `$mutWriter($stringWriter)`, which was `$mutStringWriter`, a second
   form of the same job, and a crate's writer's, and a generic `W`'s.

## Why

- **One form of a `&mut` in generic code**: a box, whatever it points at,
  as the user decided; an impl for `&mut T` and generic code agree.
- **It's what Rust writes**: `impl Trait for &mut T` is how std and
  crates let a function take a writer, an iterator or a reader by value
  or by `&mut`.
- **It's tested**: a corpus case gives a `&mut` for a type parameter to
  a trait's impl for `&mut T`, through a dictionary and where its type is
  known, takes it back from a function, derefs it through std's `Deref`
  and writes through std's `Write`, to a crate's writer, to text and
  through a generic one, against native Rust; the refusals are in the
  limitations; a mutation of each part is caught.

## Consequences

- A `&mut` to an object given for a type parameter is boxed and copied
  back, `$assign(items, items$1.value)`, where the object was given before.
- `Option<D>` of `Some(&mut v)`, which generic code passed through as the
  object, is refused, listed in [the limitations](../limitations.md).

## Amendment: a trait method's `&mut` result through a dictionary

A crate trait's `fn get(&mut self) -> &mut Self::Out`, called through a
dictionary, was read two ways: generic code that knew `Out = Vec<u32>`
took the result as the array, and generic code that didn't, as a box,
while the impl's entry gave the array. `push(&mut holder)` threw in JS
where Rust's pushed: a wrong answer. Case R
([0262](0262-when-rust-and-js-disagree.md)), now Rust's.

- **The trait's signature decides**, as it does for a function's: its
  `&mut Self::Out` is a box in generic code. A call through a dictionary
  takes `.value` of it where the caller knows the object, as a call of a
  generic function does, `generic_result_values`.
- **The entry gives that box**: an impl whose method gives the object,
  `&mut self.0` of a `Vec`, is `$cellOf(holderGet_get(arg0.value))`, a box
  a write of the whole changes in place, as `Cell::from_mut`'s is (ADR
  0153). A generic impl's own type parameter gives the box itself.
- std's `AsMut` and `DerefMut` give the box their signature says too.
- The `mut_assoc_through_dictionary` corpus case reads, pushes to and
  replaces through a crate trait's `&mut Self::Out` of a `Vec`, a number
  and a generic impl's `T`, against native Rust.

## Amendment: a `&mut` to a `&mut` of an object

rust_decimal's `impl AddAssign for &mut Decimal` has a `&mut self` that's a
`&mut &mut Decimal`, refused: a `&mut` to a value was a cell only where
that value was one, and a `&mut Decimal` is the object. A write of the
whole through it, `*r = &mut other`, makes it another, so it's a cell
too, `{ value: decimal }`, as a `&mut` to a number is. Not of an iterator
or a function, each the JS one (ADRs 0071, 0099), nor of a slice, which
may be a view of part of an array, refused as before.

Generic code holds a `Self` that's such a `&mut` in a box, as above, and
its `&mut self` in a cell of that box: each box a dictionary's entry is
given past what its method takes, a cell for each `&mut` it can't hold as
the object, is taken off, by `held_in_box`, the one question
`IntoIterator`'s entries ask too (ADR 0457). The
`mut_ref_to_object_impl` corpus case adds through such an impl where its
type is known and through generic code, against native Rust.
