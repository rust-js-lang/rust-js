# 0181. A clone of std's iterator over an array is what's left of the array

Status: Accepted. Extends [0061](0061-generic-iterators.md),
[0071](0071-stepping-iterators.md) and [0052](0052-std-trait-impls.md).

Case: A, C, D ([0262](0262-when-rust-and-js-disagree.md)).

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

## Amendment: `cycle()` of one over an array

`it.cycle()` goes over `it` and, each time it ends, over a clone of `it` as
it was when `cycle` was called. Of std's iterator over an array that clone
is what it had left, so **`cycle()` of one is `$cycle(left)`**, a generator
that goes over `left` again and again, cloning each item it owns that needs
a copy, as each round's clone does, and giving none at all where `left` is
empty, as Rust's does:

```rust
let seven: Vec<i32> = a.iter().cycle().take(7).cloned().collect();
```

```js
const seven = $cycle(a).take(7).toArray();
```

- **What a just-made `$iter(items)` has left is `items`,** as nothing's
  stepped: `stepped_left` says so, for a clone as for `cycle`.
- **A `$cycle`'s generator is a JS iterator already,** so what's chained on
  it isn't `Iterator.from(..)` first: `is_js_iterator` knows it.
- **`cycle()` of any other iterator is still an error,** a lazy one or one
  of std's adapters, as its clone is.

The `iter_cycle` corpus case runs one chained and held, of borrowed and of
owned items, one changed after, of one stepped first and of an empty one,
with native Rust; rustc's `sync-send-iterators-in-libcore` passes.
