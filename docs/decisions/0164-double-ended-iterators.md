# 0164. An iterator of the crate's from both ends, and of a known length

Status: Accepted. Extends [0055](0055-iterator.md) and
[0160](0160-user-collections.md).

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
