# 0510. `flat_map` and `flatten` of what has a destructor

Status: Accepted. Extends [0465](0465-dropping-an-adapter-is-closing-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
in what order, would change, so Rust's is kept, paid for in output.

## Context

`flat_map` and `flatten` of a source of items with a destructor were
refused. One whose inner iterators owned such items, `(0..3).flat_map(|i|
vec![Loud(i), ..]).take(1)`, compiled, and never dropped what the inner
one hadn't given. And finding them found more:

- **Two ways to lower `flat_map`**: `IterMethod::FlatMap`, `it.flatMap(f)`,
  which every call took, and `IterComb::FlatMap`, never reached, which
  alone knew a function that gives an `Option`. `flat_map(|x| if .. {
  Some(x) } else { None })` threw: JS's `flatMap` takes no number.
- **`Iterator.from(list).take(n).toArray()` was `Array.from(list).slice(0,
  n)`**: `list` stepped to its end, so a `$closing` in it was never closed.
- **Two places worked out an iterator's `Item`**: `iterator_item`, and
  `owned_items`' own projection for a `Splice`.

## Decision

**`flat_map` and `flatten` are `IterComb`'s, one way, and each holds its
source and the inner iterator it steps, both closed to drop what they
hold:**

- The source, an owning one, is `$closing`, as `zip`'s and `chain`'s are
  (`holds_its_sources`).
- Each inner iterator that owns items with a destructor is closed too:
  `(i) => $closing($iter(..), drop)`, `closed_inner`'s, `f`'s own parameter
  kept where it's an arrow of one value.
- Where both hold something, JS's `flatMap`, which closes the inner one
  first, is `$flatMap(it, each)`, which closes the source first, then the
  inner one, as Rust drops a `FlatMap`'s fields.
- A `FlatMap` or `Flatten` held, its type's drop, is `closed_on_drop`'s, of
  its source and inner iterator: `closing_both` and `closing_inner`.
- `flat_map` of a function that gives an `Option` is its `Some`s,
  `it.map(f).filter(present)`; `IterMethod::FlatMap` is gone.

And a list that mentions `$closing` isn't listed by `collected`;
`Recognition::iterator_item` is the one place an iterator's `Item` is
worked out.

## Why

- **It's the same program**: `flat_map_drops` stops inner iterators, a
  source, and both, early, holds a `FlatMap` and drops it, and flattens
  `Option`s and `Vec`s, against native Rust.
- **One way**: the `flat_map` that every call took was the one that was
  wrong; now there is one.

## Consequences

- A function given to `flat_map` that holds a value with a destructor is
  refused, as `zip`'s and `chain`'s are: its `FlatMap` isn't wrapped to
  drop it.
- Mutations close the inner iterator first, close none, refuse a source,
  take `flat_map` of `Option`s for JS's, list a closing chain, and drop
  no `FlatMap` held.
