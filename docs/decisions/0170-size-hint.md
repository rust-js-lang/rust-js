# 0170. `size_hint()` where its answer is known

Status: Accepted. Extends [0164](0164-double-ended-iterators.md).

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
| a generic `I: Iterator`'s | `$sizeHint(it)`: exact where it's an array, or one stepping through one; `[0, undefined]` of a lazy one |

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
