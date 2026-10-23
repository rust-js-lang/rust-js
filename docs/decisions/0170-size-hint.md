# 0170. `size_hint()` where its answer is known

Status: Accepted. Extends [0164](0164-double-ended-iterators.md).

Case: B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An iterator that wraps another passes its hint on, as utf8_iter's
`Utf8CharIndices` does:

```rust
fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
}
```

`size_hint()` of anything but the crate's own impl was an error, which
stopped utf8_iter, so url.

## Decision

**`size_hint()` is Rust's answer where rust-js knows it, bounds that hold
of a generic iterator, and an error elsewhere:**

| Of | JS |
|---|---|
| an iterator of the crate's that keeps std's | `[0, undefined]`, std's `(0, None)` |
| one with its own | its impl's, as before |
| std's that's an `ExactSizeIterator` over an array, `v.iter()`, `into_iter()`, `map` of one | `[n, n]`, its `len()` (ADR 0164) |
| a generic `I: Iterator`'s | `$sizeHint(it)`: exact where it steps through an array, as `$iter` gives one; `[0, undefined]` of a lazy one. (Amended: an array itself too, which a generic iterator never is; and an associated type's, smallvec's `I::IntoIter`, alike, `test/corpus/size_hint_into_iter.rs`.) |

- **A generic `I: Iterator`'s is a difference, listed:** rust-js doesn't
  keep its type (ADR 0061), so the hint is what its JS value tells. An
  array's is exact, where Rust's may be wider: an eager `filter` is an
  array of what it kept, whose Rust hint is `(0, Some(n))`. A lazy one's,
  or one of the crate's with its own `size_hint`, is `(0, None)`. Rust
  lets a hint be any bounds that hold, and these do; serde_core's
  `iterator_len_hint` sizes a buffer by it. (Amended: an error, which
  stopped serde_core; accepted as a difference.)
- **Still errors:** std's whose hint isn't exact, as `chars()`'s isn't, and
  a range's.

## Why

- **It's exact:** an `ExactSizeIterator`'s hint is its length, which std
  promises; the `size_hint` corpus case compares the crate's iterators, one
  passing another's on, and std's partly stepped, mapped and owned, with
  native Rust; a diagnostics test the refusal of `chars()`'s.
- **It's the JS a person writes:** a length, twice.

## Amendment: a range given as an `Iterator` knows its hint

A range given as an `Iterator` to generic or std code, std's `Peekable`
(ADR 0561), is the range object with `{ next: $rangeNext }`, and
`$sizeHint` found no hint of it: `(0, None)`, and `len()` panicked on
its `debug_assert` (#111). `$sizeHint` of what's given with `$rangeNext`
as its `next`, a range of numbers, is exact now, as `$rangeLen` counts
it; the dictionary stays `{ next: $rangeNext }`, as nothing else asks
it. ADR 0262's case: `len()` panicked, so Rust's. Held by
`test/corpus/peekable_range_len.rs` and the mutation
`size-hint-of-range-unknown`.

## Amendment: a `map` of the crate's iterator passes its hint on

`size_hint()` of a `map` of the crate's iterator was refused: a `map`'s
hint was known only of an array's `$iter` under it. Std's `Map` passes
its source's on, so it is now, made in place or held:

- `size_hint_of` peels each `Map` off the type as it peels each `map`
  off the JS, and asks the crate's `$iterator` under them, which has its
  impl's own `size_hint`, or gives std's `(0, None)`.
- A `$Steps`'s `map` has a `hint`, its source's, so one held in a
  variable is asked it too (`maps_the_crates_own`).
- The `$iterator` a `map` steps keeps its `size_hint`, which
  `consumed_into_iters` dropped as unasked.

ADR 0262's case: an error where Rust is fine, so Rust's. Held by
`test/corpus/adapter_size_hints.rs` and the mutations
`map-hint-of-own-refused`, `held-map-hint-refused`, `map-hint-lost` and
`map-source-hint-trimmed`.
