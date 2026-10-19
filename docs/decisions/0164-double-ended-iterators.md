# 0164. An iterator of the crate's from both ends, and of a known length

Status: Accepted. Extends [0055](0055-iterator.md) and
[0160](0160-user-collections.md). Amended: `rev()` of a lazy chain over a
slice, and `rposition` (below).

Case: N, C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An iterator of the crate's is a JS iterator of its own `next` (ADR 0055).
Collections' iterators run from both ends too, and know their length:

```rust
impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<u32> { .. }
}
impl ExactSizeIterator for Span {}
```

Each impl was an error, though it stopped 8 of the 16 crates of the crate
corpus (docs/crate-corpus.md): either's, smallvec's, anyhow's, memchr's,
utf8_iter's, arrayvec's, hashbrown's and litemap's iterators.

## Decision

**Each impl is the crate's, called where Rust calls it:**

| Rust | JS |
|---|---|
| `it.next_back()` | `spanDoubleEndedIterator_next_back(it)` |
| `it.rev()` | `$iterator(it, spanDoubleEndedIterator_next_back)` |
| `it.len()`, std's | `$exactLen(spanIterator_size_hint(it))` |

- **`rev()` is a JS iterator of its own `next_back`,** as one of its `next`
  is (ADR 0055): lazy, stepped from the back as Rust steps it, so what
  `next_back` does is seen in Rust's order. It was refused, as `rev()` of
  any lazy iterator is.
- **`len()` is its `size_hint()`'s lower bound,** where the impl keeps
  std's `len`, checked as std checks it: an upper bound that isn't the
  lower, std's own `(0, None)` among them, fails `assert_eq!`. One the impl
  writes is called.
- Neither trait has a diagnostic item: they're known by their names in
  `core`, as `Sum` is (ADR 0160).

## Why

- **It's exact:** the `double_ended_iterators` corpus case compares
  `next_back`, `rev()` collected, looped over and taken from, and `len()`
  between steps, with native Rust; `exact_len_unknown` the assertion.
- **It's the JS a person writes:** the type's own functions, stepped by a
  JS iterator.

## Amendment: `rev()` of a lazy chain over a slice

`v.iter().map(f).rev()`, `.filter(p).rev()` and `.enumerate().rev()`
were refused, as a lazy chain's closures would run in another order. Not
of these: Rust's `Map` and `Filter` call their closures from the back as
they're stepped from it, so the chain made of the items reversed calls
each on the same items in the same order, `v.toReversed().map(f)`; and an
`enumerate`'s index, which no closure of the program's makes, is made of
the items before they're reversed, `v.map((x, i) => [i, x]).toReversed()`.
Case C: what the closures print is observable, and is Rust's.
`rev_of_lazy_chains` runs each against native Rust, `take` and `next`
after it too. A chain whose closure reads an index after another
closure, or that isn't of a slice's items, is still refused.

## Amendment: `rposition`

`rposition(f)` was refused. Of a slice's items it's `$rposition(items,
f)`, each tested from the back as Rust tests them; of a `$iter` kept and
stepped from its front, of what's left, the iterator itself, not its
`$lent` wrapper, which it leaves stepped from its back to what it found,
as Rust's `next_back` leaves it. `position` and it are one method,
`positioned`. Of a lazy chain, whose closures would run in another
order, it's refused. `rposition` runs both against native Rust.

## Amendment: `rev()` of a chain over what JS steps

**`rev()` of a chain over an iterator JS steps, `Iterator.from(x)`, a
string's `char_indices()` among them, is the chain over `x`'s items,
listed and reversed**, `Array.from($charIndices(s)).toReversed().map(f)`:
each callback called from the back, as Rust's `next_back` calls it, as one
over an array's items is. writeable's KMP prefix walks a string's char
boundaries from the longest so, which was refused. A `rev()` is of a
double-ended iterator, which ends, so its items can be listed first.

## Amendment: `peekable()` of a lazy iterator

`peekable()` of a lazy chain, `(1..4).map(f).peekable()`, was refused:
made an array first, its closures would run before Rust's. Rust's
`Peekable` steps its source only when it's asked, and keeps the item
`peek` took for the next `next`. So does `$Peekable`, a JS iterator of
its source: `$peekable(chain)`, whose `peek()` steps once. `$peek`,
`$peekSome` and `$nextIf` ask it where it has no array, as `$rest` does
of a JS iterator. One of an array's items, or of a source with no
closure, `char_indices()`'s, is still its array's `$iter`.
`test/corpus/peekable_lazy.rs` peeks, takes with `next_if` and
`next_if_eq`, tokenizes, and records where its closure ran, against
native Rust.

## Amendment: `rev()` of a lazy chain over a range

`(1..4).map(f).rev()` was refused: a chain is reversed inward only over
an array's items. A range made where it's stepped, `$iterator({ start,
end }, $rangeNext)`, steps from its end too, `$rangeNextBack`, as Rust's
does by `next_back`: the chain over it reversed is the chain over the
range stepped from its end, each `map` and `filter` called from the back,
on the same numbers, in the same order. `test/corpus/rev_lazy_range.rs`
reverses chains over `a..b` and `a..=b`, and takes from one, its closure
called only as often as Rust calls it, against native Rust.

