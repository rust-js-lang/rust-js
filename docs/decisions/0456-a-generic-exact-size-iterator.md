# 0456. A generic `ExactSizeIterator` knows its length

Status: Accepted. Extends [0429](0429-a-double-ended-iterator-knows-its-back.md),
which this follows for a length as that follows for a back.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same count.

## Context

Generic code bound by `DoubleEndedIterator` is given a JS iterator that
knows its back (ADR 0429). One bound by `ExactSizeIterator` asks its
`len()`, which either's `Either` forwards, and it was refused: a JS
iterator doesn't know how many it has left.

Making it turned up a wrong answer in ADR 0429's: an iterator a variable
held, `let mut it = v.into_iter(); it.next(); last(it)`, was given as
`$iter(it)`, a `$iter` of a `$iter`, which stepped nothing: `None` where
Rust gives `Some(3)`.

## Decision

**A JS iterator that knows its length has `len()`: a `$iter`'s, what's
left of its array; a `$iterator`'s given its `len`, a range's
`$rangeLen`; a map of one, which keeps its source's. Generic code bound
by `ExactSizeIterator` is given one, as one bound by `DoubleEndedIterator`
is given one that knows its back, and its `it.len()` is its `len()`.**

1. **One answer, `knows_its_length`**, of a type parameter bound by
   `ExactSizeIterator`, a `dyn` of one, and a map of one; the caller's
   conversion, `as_exact_size`, its back too where it's double-ended.
2. **One form of `$iterator`'s arguments, `stepped_ends`**, its back and
   its length, which `stepped` is a case of.
3. **A map keeps what its source knows**, `$mapEnds`, its back and its
   length, as Rust's `Map` is double-ended and exact-sized where its
   source is (ADR 0455's `$mapBack`, renamed).
4. **`$iter` of what's a `$iter` already is itself**: a variable of an
   array's iterator holds its `$iter`, and `v.iter()` given on is `v`,
   which the compiler can't tell apart; the runtime can.
5. A crate's own iterator given as an `ExactSizeIterator` is refused yet,
   and an inclusive range's.

## Why

- **It's std's iterator**: an exact size is what `len()`, `rposition` and
  collecting into a known size ask, and either forwards it.
- **One way, ADR 0429's**: what generic code asks of an iterator travels
  with it, a method of the JS iterator, given where it's called.
- **It's tested**: a corpus case asks the length of a stepped array's
  iterator, a range's, an empty range's, a map's, and of one that's
  double-ended too, after stepping its back, and gives a variable's
  iterator on, against native Rust; a mutation of each part is caught.
