# 0990. A filter of an array's items the crate steps from its back is made knowing it

Status: Accepted. Extends [0545](0545-a-map-stepped-from-its-back-is-made-knowing-it.md)
to filters of an array's items. Its refusal in `next_back_lazy`
([0071](0071-stepping-iterators.md)) no longer holds. A gap closed, #385's
first.

Case ([0262](0262-when-rust-and-js-disagree.md)): a gap, valid Rust
refused. Its result follows Rust's: the filter's test runs on items from
the back.

## Context

```rust
let mut odd = v.iter().filter(|x| **x % 2 == 1);
odd.next_back()
```

was refused: "`next_back()` of a lazy or generic `std::iter::Filter`". So
was `next_back()` or `rev()` of a filter a struct holds. ADR 0545's
analysis, `back_stepped`, makes a map or a filter the crate steps from its
back knowing it where it's made. It left out one of an array's items,
whose own forms step it from its back. That's true of a map:
`v.iter().map(f).next_back()` is its source's last item, mapped. A
filter's form, JS's `it.filter(p)`, has no back.

## Decision

The analysis leaves out only a map of an array's items. A filter of them
the crate steps from its back is made `$filterEnds($iter(v), p)` where
it's made ([0526](0526-a-filter-steps-from-its-back.md)), held in a
variable or a struct, or not.

## Consequences

`next_back_lazy` runs and is checked against native Rust: `saw 3`, then
`Some(3)`. `held_adapters_by_ref` checks a struct's filter stepped from
its back through `by_ref()`. The mutation is `filter-of-items-native`.

`back_kept`'s rule for `by_ref()` of an adapter a struct holds is reached
by nothing: without it the corpus, the forms corpus and the snapshots are
unchanged. It's removed with its mutation, `held-by-ref-lent` (#382).
