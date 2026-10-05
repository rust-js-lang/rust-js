# 0181. A clone of std's iterator over an array is what's left of the array

Status: Accepted. Extends [0061](0061-generic-iterators.md),
[0071](0071-stepping-iterators.md) and [0052](0052-std-trait-impls.md).

## Context

An iterator that's `Clone` can be kept and gone over again, a clone at a
time:

```rust
impl<I: Iterator<Item = B> + Clone, B: Borrow<Item<'a>>> DelayedFormat<I> {
    fn write_to(&self, w: &mut impl Write) -> fmt::Result {
        for item in self.items.clone() { .. }
    }
}
DelayedFormat { items: [item].into_iter(), .. }
```

chrono formats a date so. A clone of std's iterators was an error, a
`slice::Iter`'s as an `array::IntoIter`'s, which stopped chrono.

## Decision

**std's iterator over an array, `v.iter()`, `[a, b].into_iter()` or
`v.into_iter()`, is the array (ADR 0061), which iterating doesn't change:
its clone is the array, or, of one stepped through, what's left of it:**

| Rust | JS |
|---|---|
| `v.iter().clone()`, given where an `I: Iterator + Clone` goes | `v`, itself |
| `[a, b].into_iter().clone()`, of items a clone copies | `items.map((item) => item.slice())` |
| `it.clone()` of `it` stepped through, a `$iter` (ADR 0071) | `it.items.slice(it.at)` |

- **Of an iterator that borrows its items, a `slice::Iter`, the items
  aren't copied:** they're references, as the clone's are.
- **Of one that owns them, an `array::IntoIter` or a `vec::IntoIter`, each
  that needs a copy is copied,** as the clone's `Clone` of each does: one
  changed after doesn't change the other's.
- **Still an error:** a clone of a lazy iterator (ADR 0139), a JS iterator
  that can't be gone back over, and of one of std's adapters.

## Why

- **It's exact:** the `iterator_clones` corpus case clones a `slice::Iter`,
  an `array::IntoIter` of references and of numbers, and a
  `vec::IntoIter`, kept in a generic struct and gone over twice; two
  iterators stepped through, then cloned, each stepped on its own; and the
  clone of owned items, one of them changed after, with native Rust.
- **It's the JS a person writes:** an array is gone over as often as it's
  wanted, and what's left of one is `slice(at)`.
