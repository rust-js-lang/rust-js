# 0389. A slice pattern's rest through a `&mut` is a view

Status: Accepted. Amends [0123](0123-slice-patterns.md); extends
[0335](0335-slice-views.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): every write through
the view is the slice's, as Rust's is.

## Context

A slice pattern's rest, `rest @ ..`, binds a copy of its items (ADR 0123),
as `&v[a..b]` is a copy. Through a `&mut`, or by `ref mut`, it's written
through, so it was refused, though `&mut v[a..b]` has been a view since ADR
0335. An item counted from the end, `[.., last]`, through a `&mut`, was
refused too: its place, `v[v.length - 1]`, wasn't known to stay put. Two of
rustc's tests failed on the first:

```rust
match &mut *v {
    [ref mut head, ref mut tail @ ..] => { .. }
}
```

## Decision

**A `&mut` to a slice pattern's rest is a view of those items, `$view(xs,
a, b)`, as `&mut xs[a..b]` is; and an item counted from the end is its
place, `xs[xs.length - k]`, as one from the start is `xs[k]`.**

```rust
if let [first, middle @ .., last] = &mut v[..] {
    std::mem::swap(first, last);
    bump(middle);
}
```

```js
if (v.length >= 2) {
  const middle = $view(v, 1, v.length - 1);
  const t = v[0];
  v[0] = v[v.length - 1];
  v[v.length - 1] = t;
  bump(middle);
}
```

- **Where the rest starts and ends is one rule**, `subslice_range`, of its
  copy and its view alike.
- **`xs[xs.length - k]` stays the same place** while an item of `xs` is
  borrowed: nothing can change `xs`'s length meanwhile. `from_end` says
  so, of the same `xs` on both sides, to the two questions of whether a
  place stays put, `same_place` and `fixed_place`; whether they're written
  alike is `Expr::written_alike`, which tidy's `same_expr` was.

## Why

- **It's the slice's own items**, written as Rust writes them.
- **It's tested**: the corpus's `slice_pattern_mut_rest.rs`, which was a
  compile-fail case of the refusal, sorts, bumps and reverses rests of a
  `Vec` and of arrays, swaps a first and a last item, and gives a last
  one to a function; the two rustc tests pass.
