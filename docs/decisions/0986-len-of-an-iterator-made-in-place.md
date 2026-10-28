# 0986. `len()` of an iterator made in place is counted through its adapters

Status: Accepted; its reading of a map of a `rev()`'s items, which read
a reversed range forward, is corrected by
[0987](0987-a-rev-made-in-place-read-by-its-form.md). A gap closed, found by #382's triage: `len()` of an
`ExactSizeIterator` made where it's asked, `v.iter().map(f).len()`, was
refused.

Case ([0262](0262-when-rust-and-js-disagree.md)): a gap, valid Rust
refused. Its result follows Rust's: `len()` steps nothing and calls no
adapter's function.

## Context

A std adapter held in a variable is std's compiled struct, whose
`ExactSizeIterator::len` is std's (ADR 0871). An adapter made where
`len()` is asked is what the recognizers make in place: `$iter(v).map(f)`,
`$iterator(range, $rangeNextBack)`, `$taking(..)`. Its `len()` had rules
for an array's items, a range and an array's skipped or taken items alone.
Each of these was refused, though each is an `ExactSizeIterator`:

```rust
v.iter().map(|x| x * 2).len()
(1..4u32).rev().len()
v.iter().enumerate().rev().len()
v.iter().take(2).map(|x| x + 1).len()
fn taken<I: ExactSizeIterator>(it: I) -> usize { it.take(2).len() }
```

So was a `rev()` of a map, `count(v.iter().map(f).rev())`, given to
generic code that asks its length (`as_exact_size`).

Counting by stepping, `.toArray().length`, would call each map's function,
which Rust's `len()` never does. ADR 0036 refused `len()` of a chain
whose closures do what can be seen for that reason; this decision amends
it, and its diagnostics test, "len of a chain whose closure prints", is
removed.

## Decision

`len_in_place`, beside `items_left` in `mir/iter/sources.rs`, counts an
iterator made in place by its type, as std's `len()` of each adapter asks
its source's, reading each value's form through the recognizers'
readers:

- a `map`, an `enumerate()`, a `copied()` and a `rev()`: its source's,
  where what the map is given does nothing to make;
- `take(n)`: `Math.min(len, n)`; `skip(n)`: `Math.max(len - n, 0)`, its
  source's count read before `n`, where `n` does nothing to read;
- an array's items, `$iter(xs)`: `xs.length`;
- a range of numbers: its count, as before;
- a generic one or a `dyn`'s, given knowing its length (ADR 0456): its
  own, `it.len()`, not `Iterator.from(it)`'s.

Any other form is refused, as before. `len()`'s rule of a range is this
one's.

`as_exact_size` makes a `rev()` made in place knowing its length:
- `$rev(source)`, and a range's stepped from its back: `$rev` of its
  source made knowing its back and length;
- an array's items reversed, `$iter(xs.toReversed())`: as it is;
- a map of them: as its source's map is.

A `rev()` a dictionary steps from its back, `$iterator(it, nextBack)`,
knows neither, and stays refused.

## Consequences

`v.iter().map(f).len()` is `v.length`, and `v.iter().skip(k).map(f).len()`
is `Math.max(v.length - k, 0)`. `len()`'s rule for an array's items skipped
or taken from, `counted_chain`'s `v.slice(9).length`, counted the same
construct a second way. It's removed, with its mutation,
`mir-counted-chain-len-refused`: `v.iter().take(9).len()` is
`Math.min(v.length, 9)`, as `take(2).map(f)`'s is (`len_made_in_place`,
`str_bytes`).

`test/corpus/len_made_in_place.rs` checks each form against native Rust,
including a map whose function counts its calls. The mutations are
`len-in-place-*` and `exact-size-rev-unread`,
`exact-size-reversed-items-unread` and `exact-size-reversed-map-unread`.
The case also catches #382's `rev-length-unknown`.
