# 1206. A range through std's build

Date: 2026-11-25.

Status: accepted, step S0 of S3 (Campaign 5 step 3b, #1589). Amends
[1202](1202-one-catalog-of-the-runtimes-models.md).

## Context

The runtime's range helpers, `$rangeNext`, `$rangeNth`, `$rangeNthBack`
and the others ADR 1202's catalog says are `Unproven(Ranges)`, are rust-js's
models of std's `Range` and `RangeFrom` iterators. The twin net proves a
model by a case whose plain build imports it while its std build, the
differential build, `--std-iterators`, runs std's own compiled body there
instead. For ranges, std's build couldn't compile those bodies. #1589 named
five reasons: their literal panics, their specialized private trait
`RangeIteratorImpl`, the `PartialOrd` of their bounds, each integer's
macro-written `Step`, and the recognizers treating every range as theirs.

Measured on this toolchain by giving std's build `Range`'s rows:

- A range already is std's struct in the plain build. `a..b` is
  `{ start, end }` and `a..` is `{ start }`, std's fields, so only the
  steps are rust-js's. `RangeInclusive` differs: std's holds `exhausted`,
  which rust-js's doesn't. It stays out of this decision.
- `.expect(msg)` in std's bodies is a call of its private
  `core::option::expect_failed(msg)`, `inline(never)`, which nothing
  placed: "a crate given std's bodies has no module for
  `core::option::expect_failed`".
- `Step` wasn't a trait given dictionaries, so `Range<A>`'s body had no
  `A: Step`, and none of `PartialOrd`, its supertrait.
- `RangeIteratorImpl`'s impl for a `T: TrustedStep` specializes the
  generic one, and no fact chose it.

## Decision

Std's build compiles `Range`'s and `RangeFrom`'s `Iterator` and
`DoubleEndedIterator`, std's own `Step` of each integer and `char`, and
`RangeIteratorImpl`'s two impls, in steps:

- **S0.** `expect_failed(msg)` is `panic_display(&msg)` behind its
  `#[track_caller]`, its payload a `String` (ADR 0660). It's recognized as
  that, `Std::PanicDisplay`, by one predicate, `std_items::is_expect_failed`.
  `classify_fn` asks it, and so does compiled std's walk, which compiles
  every private callee of a compiled body, as no program can name one:
  this one a recognizer answers, as `io`'s answered functions are. What
  `panic_display` shows is its argument, `x: &T` or `msg: &str`, seen
  through its reference, `tys[0]`, the same rule for both. A consumer's
  copy of a library's `.expect(..)` lowers from the library's own body,
  `$unwrap`, so only std's build reaches it.
