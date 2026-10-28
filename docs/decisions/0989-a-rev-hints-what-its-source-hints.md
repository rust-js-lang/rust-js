# 0989. A `rev()` hints what its source hints

Status: Accepted. Narrows the difference
[0170](0170-size-hint.md) lists, a generic iterator's lazy hint, by one
form. Found by #382's triage of `size-hint-not-what-it-steps`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, within a
listed difference. Rust lets a hint be any bounds that hold, so `(0, None)`
was allowed; std's `Rev` gives its source's, which is now given too.

## Context

```rust
fn reversed_hint<I: DoubleEndedIterator>(it: I) -> (usize, Option<usize>) {
    it.rev().size_hint()
}
reversed_hint(v.iter())
```

A `rev()` generic code makes of what it's given is `$rev(it)`. `$rev` kept
no hint, so `$sizeHint($rev(it))` was `(0, None)`, where Rust gives
`(3, Some(3))`: `Rev::size_hint` is its source's.

## Decision

`$rev`'s hint is its source's, `$sizeHint(it)`, as std's `Rev`'s is.

## Consequences

No snapshot changed. `test/corpus/hint_through_a_given_value.rs` checks
it against native Rust, with the mutation `rev-hint-unpassed`. The case
also checks a type parameter given as it is that's passed on to generic
code, which catches #382's `size-hint-not-what-it-steps`.
