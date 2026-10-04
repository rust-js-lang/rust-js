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

**`size_hint()` is Rust's answer where rust-js knows it, and an error
elsewhere:**

| Of | JS |
|---|---|
| an iterator of the crate's that keeps std's | `[0, undefined]`, std's `(0, None)` |
| one with its own | its impl's, as before |
| std's that's an `ExactSizeIterator` over an array, `v.iter()`, `into_iter()`, `map` of one | `[n, n]`, its `len()` (ADR 0164) |

- **Still errors:** std's whose hint isn't exact, as `chars()`'s isn't; a
  range's; and a generic `I: Iterator`'s, whose type rust-js doesn't keep,
  so can't give Rust's answer for, as serde_core's `iterator_len_hint` is.

## Why

- **It's exact:** an `ExactSizeIterator`'s hint is its length, which std
  promises; the `size_hint` corpus case compares the crate's iterators, one
  passing another's on, and std's partly stepped, mapped and owned, with
  native Rust; a diagnostics test the refusal of `chars()`'s.
- **It's the JS a person writes:** a length, twice.
