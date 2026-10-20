# 0531. `try_rfold` is `try_fold` from the back

Status: Accepted. Extends [0455](0455-stepping-from-the-back.md) and
[0458](0458-try-fold.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the items folded, the
order, where it stops, and what's left are Rust's.

## Context

`try_rfold` was refused ("calling `DoubleEndedIterator::try_rfold`"),
itertools' `process_results` among its callers.

## Decision

**`try_rfold(init, f)` is `try_fold` of the iterator reversed:
`$tryFoldOk($rev(it), init, f)` or `$tryFoldSome`.** It steps what knows
its back as `rfold` does, `back_step`'s, and `IterMethod::steps_from_its_back`
is the one answer to which methods do. What it stops before is left in
the iterator, as Rust's is.

## Why

`try_rfold` folds generic code's iterator into a `Result`, stopping at an
`Err`, and an array's into an `Option`, stopping at a `None` and reading
what's left, against native Rust.

## Consequences

Mutations stop seeing it as stepped from the back, and fold it forward.
