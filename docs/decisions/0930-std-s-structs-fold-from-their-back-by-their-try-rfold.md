# 0930. std's structs fold from their back by their own compiled `try_rfold`, given where they're given and where they're held

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0913](0913-std-s-generic-try-is-a-dictionary.md) and
[0927](0927-skip-and-take-fold-from-their-back-as-std-s.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A
held `count().take(5).skip(1)`'s `rfind(p)` of the crate's iterator with
its own `nth_back` asked it `nth_back(4)` and `nth_back(0)`, stepping from
the back, where Rust's `rfind`, std's `Skip::try_rfold` of `Take::try_rfold`,
asks `nth_back(3)` once; with std's structs alone, so did `last()` of a
`rev()` of one, and `sum()` of a `rev().take(2)`.

## Context

What folds from the back by `try_rfold`, `rfind`, a `rev()`'s `try_fold`
and so `any`, `find`, `sum` of a `rev().take(n)`, reaches each adapter's
own `try_rfold`, which `Take`'s, `Skip`'s and `StepBy`'s write to step the
crate's iterator by `nth_back`. None was compiled but `Skip`'s and
`Take`'s (ADR 0927), and none was reachable: a std struct's
`DoubleEndedIterator` dictionary carried `next_back`, `rfold` and
`nth_back`, `$given` gave `$iterator` those alone, and a struct a use
site made knowing its back, `$knowing`, took no `try_rfold`. So std's
default `try_rfold`, of `next_back`, ran.

## Decision

- **Each adapter whose `rfold` std compiles has its `try_rfold` compiled
  too**: `Rev`'s and its `try_fold`, `Map`'s, `Enumerate`'s, `Inspect`'s,
  `Fuse`'s, `Peekable`'s, `Flatten`'s, `FlatMap`'s, `StepBy`'s and
  `Chain`'s.
- **A std struct's compiled `try_rfold` is given wherever its back is**,
  as a JS iterator's `tryRfold(init, f, broke)` of `$tryOf(broke)`, as
  `tryFold` is (ADR 0913): in its `DoubleEndedIterator` dictionary, which
  `$given` passes to `$iterator`, and in what a use site makes it know,
  `$knowing(..)`, which `rfind`'s `$findBack` and a `rev()`'s `tryFold`
  ask.

## Consequences

- `held_back_folds_by_try_rfold.rs` runs as native Rust does, in the
  normal build and with std's structs alone, and joins the forms suite's
  few; ADR 0908's `differs` is 15, from 16 (`own_back_through_adapters.rs`).
- The JS grows by what carries a `try_rfold` it may not be asked: 15
  cases' snapshots, 457 lines in, 26 out, each a `tryRfold` entry or a
  sixth `$knowing` argument. Readability work left open: give a held
  struct only the back methods its use asks.
- Mutations: `knowing-try-rfold-ungiven`, `dictionary-try-rfold-ungiven`,
  `given-try-rfold-dropped`, `knowing-try-rfold-uninstalled`.
