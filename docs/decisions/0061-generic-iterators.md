# 0061. `impl Iterator` is the type it hides; a generic iterator is any JS iterable

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md) and [0055](0055-iterator.md).
Amended by [0421](0421-std-associated-type-drops-are-given.md): the JS
iterator made of one of the crate's, `$iterator(value, next)`, has `of`,
`value`, which its drop drops; and a trait impl's method is given one, as
a function is.
Amended by [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md):
a parameter an impl's type holds, or a signature has in anything but
itself, is given as it is, and stepped by its `Iterator` dictionary.
Amended by [0429](0429-a-double-ended-iterator-knows-its-back.md): one
bound by `DoubleEndedIterator` is a JS iterator that knows its back.

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An iterator has two JS forms:
- a std one over a slice, a `Vec` or a range is an array (ADR 0036);
- one of the crate's own is a JS iterator, made by `$iterator` (ADR 0055).

Neither covered the two ways Rust code hands iterators around:

```rust
fn evens(n: u32) -> impl Iterator<Item = u32> { (0..n).filter(|x| x % 2 == 0) }
fn total<I: Iterator<Item = u32>>(items: I) -> u32 { items.sum() }
```

An `impl Iterator` was an error: an opaque type is none of the types
rust-js knows. A generic `I` could be either form, depending on the caller.

## Decision

- **An `impl Trait` type is the type it hides.** Once type checking is
  done, rustc knows what `evens` returns, a `Filter` of a range, and
  rust-js reveals it wherever it decides by type: an array here, `$range(0,
  n).filter(..)`. An `impl Iterator` hiding one of the crate's own
  iterators is that struct, as it is in Rust.
- **Generic code takes an iterator with `Iterator.from(items)`.** It
  accepts an array and a JS iterator alike, and gives JS's lazy helpers
  (ADR 0055): `Iterator.from(items).drop(1).take(k).toArray()`. A `for`
  over an `I: IntoIterator` is `for (const x of items)`, which also takes
  either.
- **One of the crate's own iterators, given where a generic one goes, is
  made a JS iterator first:** `total($iterator({ n }, countdownIterator_next))`.
  An array needs nothing.

## Why

- **It's the same JS either way.** A caller's array or iterator reaches
  generic code as what it is, with no dictionary: `Iterator.from` does the
  one thing that differs.
- **Revealing the type is exact.** It's the type Rust itself uses when it
  generates code.

## Consequences

- An `impl Iterator` is the type it hides wherever its JS is asked for, a
  `&mut` to one too, as `next()`, `any` and `find` take it: `tens(v).next()`
  is `tens(v)[0]`. (Amended: a `&mut` to one a function returned was a box,
  so `next()` of it was `None`, and `any` of it a `TypeError`.)
- An associated type bound by `Iterator` where its trait declares it,
  `type Iter: Iterator`, is a generic iterator as a type parameter bound
  so is: a `&mut` to one, `advance(&mut items)`, is the JS iterator it
  steps. (Amended: only where-clauses were read, so a `&mut` to one was a
  box, and what stepped it was given the box, a `TypeError`;
  `generic_associated_types` runs it against native Rust.) And an impl's
  value for one, a range or an iterator of the crate's, `type Iter =
  Range<u32>`, is made a JS iterator where its trait's dictionary gives
  it, as one given where a generic iterator goes is; a call of the impl's
  own method still gives the range. (Amended: the range itself, which
  generic code's `for .. of` threw on.)
- `next()` on a generic iterator is still an error. Generic code uses the
  adapters, `for`, and what consumes one.
- A range handed over as a value (`total(0..3)`) is still an error, as a
  range in a variable is (ADR 0036).

## Amendment: a type parameter's associated type too

`i.into_iter()` of `H::Item: IntoIterator`, itertools'
`multi_cartesian_product`'s, was refused: only a type parameter was
taken for what its caller gives. An associated type only a caller knows
is the same, an array or a JS iterator, stepped by `Iterator.from`:
`is_given_iterable` answers it, for `into_iter`. A generic iterator's
value of an associated type stays as it is: a dictionary gives it, a JS
iterator already.
`test/corpus/associated_into_iter.rs` steps them against native Rust.

