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

## Amendment: the crate's own, made knowing where it's asked

The crate's iterator whose type is a `DoubleEndedIterator` or an
`ExactSizeIterator` was a JS iterator of its `next` alone, so
`enumerate().rev()` and `rposition` of one were refused (#165). Where its
back or its length is asked, it's made knowing them,
`user_iterator_knowing`: its `next_back`, and its length its own
`size_hint`'s, which `$exactLen` checks is exact as std's `len()` does.
`$rposition` of one steps it from its back, as std's `try_rfold` does.
Where nothing asks, it's made as it was, and given as it was to generic
code. A `map` of one whose `len()` is asked is refused still. Held by
`test/corpus/user_iterator_ends.rs`.

## Amendment: `rposition` of what `as_exact_size` makes knowing both

`rposition` of a lazy iterator asked its type alone whether it knew its
back and its length, and refused `v.iter().map(f).rposition(p)`, though
`as_exact_size` makes a map made in place `$mapEnds` of the array's
`$iter`. `rposition` now asks `as_exact_size` alone, which makes it
knowing both or refuses: a map, a copy, a `rev`, a string's bytes, the
crate's own, and an `enumerate` of an array's items made in place,
`$enumerate` of their `$iter`, as `as_double_ended` makes it. A map held in
a variable, a `skip`, a `take`, a map of a range or of the crate's
iterator stay refused. ADR 0262's case: C, an error where Rust is fine.
Held by `test/corpus/rposition_of_lazy.rs` and the mutations
`rposition-of-lazy-refused`, `exact-size-enumerate-unmade` and
`rposition-of-user-refused`.

## Amendment: a `copied()` that knows its length is made knowing it

`knows_its_length` says a `copied()` knows its length where its source
does, and a type parameter given as it is (ADR 0411) where its bounds give
`ExactSizeIterator`. But the `copied()` of such a parameter was the JS
iterator `$given(it, IIterator)` made without its `len`, and a copy of
items copied as it steps, structs something changes, was JS's `map` of
them, which knows no length either, so `copied.len()` threw where Rust
gives the count (#387, a wrong answer). Now a `copied()` whose type knows
its length is made knowing it: of a parameter given as it is, `$given(it,
IIterator, false, undefined, IExactSizeIterator.len)`; of items copied as
it steps, `$mapEnds(it, (item) => ({ ...item }))`, whose `len` is its
source's. A copy of one whose length nothing can ask is made as before.
ADR 0262's case: C, Rust's behavior preserved. Held by
`test/corpus/copied_given_len.rs` (the crate's own iterator and a slice's,
given as they are and not, lent, and of structs copied as it steps) and
the mutations `given-copied-lengthless` and
`copied-items-mapped-lengthless`.

## Amendment: a `copied()` that knows its back is made knowing it (#850)

ADR 0262's case: results change, so Rust's behavior is kept. A
`cloned()` of items with a `Clone` of their own, a map of each clone, was
`$mapEnds` only where its source knew its length, as #387 asked. A chain
the crate steps from its back elsewhere is made knowing its back
(`$lazyChain`) but knows no length, so its `cloned()` was `.map(clone)`,
`$Steps`' map with no back, while `knows_its_back` said the copy knows its
back, as a copy is its source (ADR 0480): `$foldBack` called its missing
`back` and threw where Rust folds it. A copy now asks what a map asks,
its back or its length (`copied_items`' `ends`). Found by #818's rows
beside `vec iter | chain.map | rfold`, the chain's type made stepped from
its back by it; alone, the row is refused. Corpus
`cloned_of_a_chain_stepped_from_its_back`; mutation
`copied-items-mapped-backless`, and `copied-items-mapped-lengthless`
retargeted. That the back step trusts the type, not the value's form, is
#851's: one per-value fact for maps, chains, zips, enumerations, flattens
and copies, as #814 made for counted adapters.

