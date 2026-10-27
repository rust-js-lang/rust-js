# 0982. A lent iterator keeps its back and its length

Status: Accepted. Extends [0071](0071-stepping-iterators.md)'s `$lent` and
[0928](0928-a-mut-i-folds-from-its-back-by-try-rfold.md): std's `&mut I`
passes on an `I: DoubleEndedIterator`'s back, folding it by `try_rfold`,
in JS too. Found by #382's triage.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The
program threw where Rust gives the last item.

## Context

```rust
fn last_through<'a, R: DoubleEndedIterator<Item = &'a u8>>(r: &mut R) -> Option<u8> {
    r.copied().next_back()
}
```

`r` is lent, `$lent(Iterator.from(r))`, and a `copied()` is its source in
JS, so `next_back()` of it is `$nextBack($lent(..))`. `$lent` passed on
its iterator's `nth`, `advance_by` and `try_fold`, as std's `&mut I` does,
but none of its back: `TypeError: it.back is not a function`, where Rust
gives `Some(3)`. A probe of one of #382's survivors,
`adapter-impl-asks-unread`, found it.

## Decision

`$lent(it)` passes on `it`'s `back`, `nthBack`, `tryRfold` and `len`, as
std's `&mut I` passes on an `I: DoubleEndedIterator`'s `next_back`,
`nth_back` and `try_rfold` and an `ExactSizeIterator`'s `len`. It doesn't
pass on `rfold`: std's `&mut I` folds from its back by `I`'s `try_rfold`,
as its dictionary does (ADR 0928).

## Consequences

`test/corpus/copied_of_a_parameter.rs` checks a `copied()` of a type
parameter stepped from its back and counted, directly and through a
`&mut`. The mutation is `lent-back-unpassed`.
