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

## Amendment: a `by_ref()` lends, never owns, on every path (2026-11-05, #745, #747, #749, #755)

Case R: each was a wrong answer, a `NaN`, a hint of `(0, None)`, the
crate's own `fold` or `rfold` run and its iterator dropped twice, or what
its owner still held dropped.

Std's `&mut I` (`core/src/iter/traits/iterator.rs:4233`,
`double_ended.rs:437`) passes on `I`'s `next`, `size_hint`, `advance_by`,
`nth` and `try_fold`, its `next_back`, `advance_back_by`, `nth_back` and
`try_rfold`, and its `len`. Its `fold` and `rfold` are `I`'s `try_fold` and
`try_rfold`, so neither reaches `I`'s own, which takes `I` by value. ADR
1092 §3's `&mut I` row says the same. `$lent` is the one owner of that,
and every `by_ref()` path now goes through it:

- **`$lent` passes on `size_hint`** (#745): `lent.sizeHint` is `it`'s, by
  `$sizeHint`, as it passed on `len` already.
- **A `len()` of a `&mut`** is the lent value's `len()`, not `it.end -
  it.at` of a `$iter` read off a value that isn't one (#745).
- **A held reversed range,** std's compiled `Rev`, is no form of its range:
  `len_in_place` counts a reversal only of `$rev(x)`, a `$iter` or a
  range's stepped form, and otherwise asks `$sizeHint` (#745).
- **`size_hint()` of a `&mut` to the crate's iterator** is its impl's own,
  or std's default, as its `next` was (#745).
- **`rfold` of a `&mut`** is `$foldBack($lent(it), ..)`: the crate's
  `next_back`, never its own `rfold` (#747). The forward fold was lent
  already.
- **`by_ref()` of what knows its back,** a type parameter's, is lent, not
  the value itself, so generic code given it, `Total::sum(held.by_ref())`,
  folds it by `try_fold` and never drops it (#749).
- **A `map()` of a `&mut`** that knows its back or its length, a type
  parameter's, is closed by the receiver's own type, a `&mut`, so never:
  a collect of a `take(2)` of it dropped what its owner still held (#749).
- **`rev()` of a `&mut`** steps a lent value from its back and is never
  closed: a `Rev<&mut I>` drops nothing of `I` (#755). The `rev()` lowering
  asked `closing` of the receiver's type with its references peeled.
- **A lent `Vec`'s `IntoIter` keeps its slice's steps** (#755). `$lent(it)`
  keeps what it lends as `lends`; `$slicing` of a lent value gives `lends`
  its slice's steps and passes them on, in place, as `$given` and the
  lowering's `slicing` ask it, of whatever form it's given in.

The step is named `sizeHint` where it was `hint`, the name proto's design
of ADR 1092 gives it; a compiled struct's `steps.hint` is unchanged, so no
output changes.

`test/corpus/by_ref_lends.rs` checks each, natively and in both builds
(it joins iterator-forms' few). The mutations are `lent-hint-unpassed`,
`lent-lends-unsaid`, `slicing-lent-unpassed`, `slicing-lent-ignored`,
`by-ref-knowing-unlent`, `rfold-of-mut-unlent`, `len-of-mut-items-left`,
`hint-of-mut-own-refused`, `rev-of-mut-closed`, `rev-of-mut-unsliced`,
`map-of-mut-closed` and `len-in-place-held-rev-read`. ADR 1092's conformance table holds it too:
its 66 `by_ref` case-builds under #745, #747 and #749 are taken off. Its
three left, a generic `by_ref().map(..)` collected into a `String`, a
`HashSet` or a `HashMap`, which never asks the crate's `size_hint`, are
#733's.
