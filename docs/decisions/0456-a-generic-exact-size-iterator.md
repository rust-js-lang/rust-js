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


## Amendment (2026-11-18): an adapter made already asks its source's length (#2052)

`map(Some)` of what can't look like `None` is no map, its source as it is
(#1342), but not where a `Box` holds one of its type: then `as_double_ended`
made it `$mapEnds` of a range made for its back alone, `$iterator(range,
$rangeNext, false, $rangeNextBack)`, with no `$rangeLen`. An `enumerate()`
of it stepped from its back, whose `next_back` counts by its source's
`len()`, was given it as it was, as it had a back, and `as_exact_size` took
it for one that knows its length by its type, so the JS threw
`enumerated.iter.len is not a function` where native ran (the metamorphic
net's `boxed` rewrite of `as_is_map_stepped_from_its_back`).

One rule now, by the form's question beside `form_lacks_a_back`:
`form_lacks_a_length` says a form knows no length, down through what
passes its source's on (`passes_its_length_on`: what holds or lends one as
it is, `$held`, `$lent` and `$closing`, and a map's and an enumeration's,
`$mapEnds` and `$enumerate`, as std's `Map` and `Enumerate` are
`ExactSizeIterator`s where their source is) to an `$iterator` its maker
gave no `len`. `as_exact_size` makes such a form's source know its length
and puts it back in place. And `backs.rs` asks `as_exact_size` of an
enumeration's source stepped from its back whether or not it has a back, as
`Enumerate::next_back` counts by its length either way. A filter knows no
length, and a `take`'s and a `skip`'s count their own, which their own
branches make.

Corpus case `map_as_it_is_boxed_beside_an_enumeration_from_its_back`,
against native (it throws at main); the `boxed` rewrite leaves
`test/metamorphic-known.txt`. Mutations `length-passed-on-unasked` and
`enumerated-from-its-back-length-unasked`, caught;
`enumerate-of-take-from-back-unknowing` re-anchored.


## Amendment (2026-11-18): an iterator's own `len` is its dictionary's (#2073)

A dictionary entry of a trait's provided method is the impl's own wherever
the impl writes one; `own_methods` gave generic code a crate iterator's own
`fold`, `nth`, `last`, `rfold`, `nth_back` and `count`, but its `len` was
always std's default over its `size_hint`, `(it) => $exactLen(hint(it))`.
An iterator that writes `len` and keeps std's `size_hint` was given to
`fn len_of<I: ExactSizeIterator>` with a `len` of `$exactLen([0,
undefined])`, which panicked `assertion left == right failed` where
native gives its own length; one whose hint and `len` disagree got the
hint's. Results changed, so Rust's behavior is kept (ADR 0262).

One answer now, `user_len`: the impl's own `len` (`own_len`, beside
`own_iterator_method`) where it writes one, else the hint's. Each place
that gives the crate's exact-sized iterator its `len` asks it: made
knowing its length (`user_iterator_knowing`, and reversed,
`reversed_user_knowing`) and given as it is (`iterator_extension_dictionary`'s
it, which used a second form of the hint's, `(iterator) => ...`; now
`exact_len_by`'s one form).

Corpus case `own_provided_methods_given_through_bounds` gives each of the
eight provided methods a user impl commonly writes, `len`, `size_hint`,
`count`, `last`, `nth`, `nth_back`, `fold` and `rfold`, to generic code
by value, through a `&mut` and reversed, against native; at main only
`len` differed, in all three. Mutations `own-len-unasked` and
`dictionary-len-of-own-by-hint`, caught.

## Amendment: `rposition` folds from the back by the iterator's own `try_rfold` (#2142)

`$rposition` of a JS iterator that knows its back and its length asked
its `len()`, then stepped `back()` one item at a time. Std's `rposition`
is `let n = self.len(); self.try_rfold(n, ..)`, so the iterator's own
`try_rfold` is what's asked. They differ where an adapter's `try_rfold`
asks its source otherwise than its `next_back` does: `Enumerate`'s asks
its source's length once, its `next_back` at every step. So
`src.enumerate().rposition(p)` of a generic `I` printed `hint back hint
back ..` where native prints `hint hint back back ..` (found by the
tables, ioDC). Now it's the iterator's length, then `$tryRfoldOf`: its
own `try_rfold` where it has one, a compiled `Enumerate`'s among them,
else std's default of its `next_back`. A slice's items and a `$iter`
stay as they were. ADR 0262's case: results change, Rust's behavior
kept. Held by `test/corpus/rposition_asks_try_rfold.rs` (a generic `I`'s
enumeration, the crate's own iterator's `rposition`, and a `&mut` of one,
in both builds) and the mutation `rposition-of-a-js-iterator-by-next-back`.

The steps' catalog's probe, test/std-steps.rs, now asks each adapter
std's consumers whose bodies ask particular steps of it: `position`,
`find`, `find_map`, `any`, `all`, `max_by` and `min_by`, from the front;
`rfind`, from the back; and `rposition`, of one that's also exact-sized;
each a predicate that holds at the second item, so what a consumer asks
as it stops is seen. test/iterator-steps.test.ts holds each of the
runtime's forms to them, each asked as the lowering asks it; every form
asks what std's does.

## Amendment (2026-11-20, #2193, #2194): each form has std's `len`, and no type claim stands for a form that lacks one

Generic code bound by `ExactSizeIterator` asks `it.len()` of what it's
given. The argument's lowering (`given_iterators`) runs `as_exact_size`,
which returned the value as it was wherever `knows_its_length_of` said
yes. That answer starts from a claim about the type, `knows_its_length`,
so whether the form had a `len` depended on a crate-wide fact of its
type made elsewhere. Two such facts led to forms without one:

- **#2194.** A `capacity()` read marks `Take<Fuse<Range>>` counted
  (hint_asked.rs's `counted_in`). Its maker then made
  `$taking(iterator_fuse(..), 3)`, with no `$knowing` on the compiled
  `Fuse`. `$taking` defined a `len` only where its source had one, so
  `hint_of(..)` threw `it.len is not a function`. An `rfold` of it threw
  too: `$taking`'s back needs its source's `len` and back.
- **#2193.** A boxed `rev()` of `Zip<Count, Range>` made the zip
  `$zipEnds` crate-wide, and the `inspect` over it JS's own map,
  `$Steps.map`, which had no `len`.

The fix is in the forms, which now answer `len` as std's adapters do.
The runtime net's catalog lists `len` for each of them:

- `$taking`, `$stepBy` and `$lazyZip`: always their hint's lower bound.
  std's `Take`, `StepBy` and `Zip` write no `len`, so theirs is the
  default, by their own `size_hint`. A zip's length no longer asks its
  sides' `len`, which the crate's own may write; std's asks their
  `size_hint`s. One std reads by index, `$trustedZip`'s, keeps the
  shorter side's, which no crate code can tell apart.
- `$Steps.map`: its source's `len`, where the source has one, as std's
  `Map` writes it. This form serves `inspect` and `cloned` too.

The net (test/iterator-steps.test.ts) no longer lets a form off its
length because it has no `len`; that is what hid both. Where std's length
asks no `len` of a source that writes one (Take's, Skip's, StepBy's,
Zip's, Peekable's), it also asks the form's length over sources with no
`len`. That is where `$taking` of a compiled `Fuse` lacked one. Against
the old runtime it fails seven forms.

In the compiler, `as_exact_size` doesn't take a form for knowing what it
shows it lacks:
- A `take(n)` or `skip(n)` made already, `$taking`/`$skipping` of a
  source whose form lacks the `len` and back its back steps
  (`counted_lacks_its_back`), is rebuilt with its source made knowing
  them. `counted_parts` now reads that helper form too.
- A value whose form shows no length (`form_lacks_a_length`) is never
  returned as it is.
- A `debug_assert!` holds `knows_its_length`'s claim to the form: where
  the form shows no length, the type mustn't claim one. Every corpus and
  table run checks it.

Corpus `exact_size_bound_given_any_form` holds both issues' programs, a
`peekable().take(3)`, a `step_by`, a bound of `ExactSizeIterator` alone (whose `len` is `$taking`'s own), the `rfold`s by value of a holding closure, and a
zip's `len` of a crate iterator that writes its own `len`. Mutations:
`taking-len-of-its-source-alone` (runtime/taking.ts), `steps-map-lenless`
(runtime/steps.ts), `step-by-len-of-its-back-alone`
(runtime/back_steps.ts), `lazy-zip-len-of-its-sides-lens`
(runtime/lazy_zip.ts), `counted-back-of-its-source-unchecked` and
`counted-helper-parts-unread` (lower/iterators/knowing.ts);
`zip-ends-len-longer` now mutates the by-index zip's length.

Case (ADR 0262): R. A throw where Rust prints, and which of the crate's
steps `len` asks.
