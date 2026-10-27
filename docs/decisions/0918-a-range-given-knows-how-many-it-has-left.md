# 0918. A range given to generic code knows how many it has left, and std's struct's default `len` is its exact `size_hint`

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0917](0917-a-held-step-by-is-std-s.md),
[0456](0456-a-generic-exact-size-iterator.md) and [0170](0170-size-hint.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
`size_hint()` of a range given to generic code, `hint(2..7)`, was
`(0, None)` where Rust's is `(5, Some(5))`; and with std's structs alone,
a `rev()` of a `step_by()` over an `a..=b`, or of a `take()` over a
`skip()`, was refused.

## Context

A range given to generic code is a JS iterator, `$lazyRange(2, 7)` written
in place or `$iterator(r, $rangeInclusiveNext)` of a variable, and
`$sizeHint` of either knew nothing of its bounds; only `{ next: $rangeNext }`
given as it is, a range of std's own code, was exact. An `a..=b`'s
`ExactSizeIterator` dictionary didn't exist. And std's `Skip` has no `len`
of its own: `ExactSizeIterator::len`'s default is its exact `size_hint`,
which neither a dictionary nor what a use site gives std's struct asked.

## Decision

- **A range given to generic code is sized by its bounds**: `$lazyRange`'s
  `hint`, of its start, end and step, and `$iterator`'s of a range's step,
  `$rangeHintOf(next)`, beside its `nth`, `$rangeNthOf(next)`; an
  inclusive range given as it is is exact too.
- **An `a..=b`'s `ExactSizeIterator` is `$rangeInclusiveLen`**, which
  rustc gives only 8- and 16-bit numbers, JS's own numbers.
- **The `len` of std's struct whose `len` is std's default is its exact
  `size_hint`**, `$exactLen($sizeHint(iterator))` (`default_len`), in its
  dictionary and where a use site gives its length.

## Consequences

- `range_hints_given.rs` runs as native Rust does, with std's structs
  alone too; ADR 0908's `differs` is 24, from 26 (`rev_counted.rs`,
  `rev_step_by.rs`).
- `held_adapters_by_ref.js`: a reversed held `skip()` is given its length.
- Open: an endless `n..` given to generic code is still `(0, None)`, Rust's
  `(usize::MAX, None)`.
- Mutations: `range-hint-unknown`, `lazy-range-hint-unknown`,
  `inclusive-range-hint-unknown`, `inclusive-range-len-refused`,
  `default-len-refused`, `held-default-len-unknown`.

## Amendment: a range's `step_by` knows it too

A wrong answer: a range's `step_by` given to generic code as `$stepBy`, a
range held in a variable, `r.step_by(3)`, or of bounds tidy can't read,
said `(0, None)` of its size where std's `StepBy` says its range's exact
length stepped, `(4, Some(4))` of ten by three. `$stepBy`'s `hint` is
std's now, of its source's: before its first item is taken, that one and
each `step`th after, then each `step`th. Found by tidy's stages (ADR
0659): `range_hints_given`'s `(0..10).step_by(3)` is `$stepBy` until the
`range steps` pass folds it. Held by `step_by_range_size_hint.rs`,
`range_hints_given.rs` at every stage, and `step-by-hint-unknown`.
