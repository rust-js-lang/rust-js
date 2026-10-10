# 0336. A slice's prefixes compare its items by their own `==`

Status: Accepted. Extends [0324](0324-slice-methods.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A slice's `starts_with`, `ends_with`, `strip_prefix` and `strip_suffix`
compared items with JS's `===`, so they were taken only for items whose
`==` that is: numbers, text and `bool`s. Of structs, of a type with its
own `PartialEq`, or of a generic `T: PartialEq`, they were errors, as was
`strip_circumfix`, stable since Rust 1.98. `contains` already compared
other items by their `==`.

## Decision

**Each compares its items by their own `==`: JS's `===` where that's it,
or else the function `==` lowers to, given to the helper.**

```rust
path.starts_with(&origin)
items.strip_circumfix(edge, edge)
```

```js
$sliceStartsWith(path, origin, false, $eq)
$sliceStripCircumfix(items, edge, edge, (a, b) => TPartialEq.eq(a, b))
```

| Rust | JS |
|---|---|
| `v.starts_with(p)`, `ends_with(p)` | `$sliceStartsWith(v, p)`, `$sliceStartsWith(v, p, true)`, then `eq` if it isn't `===` |
| `strip_prefix(p)`, `strip_suffix(p)` | `$sliceStrip(v, p, end, eq)` |
| `strip_circumfix(p, s)` | `$sliceStripCircumfix(v, p, s, eq)`: `strip_prefix`'s, then `strip_suffix`'s of what's left |

- **A function that's `==` itself is given as it is**: `(a, b) => $eq(a,
  b)` is `$eq`, as the helper gives it two items alone. A dictionary's
  method keeps its arrow, as `this` may matter to it.
- **A float compares by `===`**, as Rust's `==` does: `NaN` isn't equal to
  itself in either.

## Why

- **It's the same program**: a derived `==`, a hand-written one that isn't
  structural, and a caller's dictionary decide each, as Rust's do.
- **It's the JS a person writes**: one call, given how to compare.
- **It's tested**: the `slice_prefix_eq` corpus case runs, against native
  Rust, each of them of a derived and a hand-written `==`, a generic one,
  and `strip_circumfix` whose prefix and suffix would overlap. Mutations
  compare by identity, drop `eq` from each helper and call, keep the arrow,
  take `ends_with` for `starts_with` and keep the suffix.
- `docs/std-coverage.txt`: `slice` 96 of 133.
