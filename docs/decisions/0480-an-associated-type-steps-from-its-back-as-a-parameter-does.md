# 0480. An associated type steps from its back as a type parameter does

Status: Accepted. Extends [0429](0429-a-double-ended-iterator-knows-its-back.md),
[0456](0456-a-generic-exact-size-iterator.md) and
[0474](0474-an-iterator-another-bound-reads-is-given-as-it-is.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same items, from
the same end.

## Context

A type parameter bound by `DoubleEndedIterator` or `ExactSizeIterator` is
a JS iterator that knows its back or its length, its caller's (ADRs 0429,
0456). An associated type a bound says is one, `I::IntoIter:
DoubleEndedIterator`, wasn't: `iterable.into_iter().rev()`, itertools'
`rev` and `partition`, was refused. So was a `Copied` of a slice's iterator
given as one, `[1, 2].iter().copied()`. And ADR 0474 gave a parameter as it
is where a bound has methods, `DoubleEndedIterator`'s too, which its JS
iterator answers.

## Decision

- **An associated type a bound says is double-ended, or exact-sized, is a
  JS iterator that knows its back, or its length**, as a type parameter's
  is: `extension_param`'s, of either.
- **A `Copied` of one is the one it copies**, whose JS iterator it is: it
  knows its back and its length where that does, and is given as that is.
- **An iterator's extension trait doesn't make a parameter given as it
  is** (ADR 0474): its JS iterator answers its methods.

```js
function rev_all(iterable) {
  return $iterator($intoIter(iterable), $nextBackSome, true);
}

last_even($iter([1, 2, 3, 4, 5]));
```

## Why

- **It's std's**: `associated_double_ended` reverses a generic iterator and an
  `I::IntoIter`, finds from the back of a `Copied`, and takes the last of an
  `Iterator + DoubleEndedIterator`, with native Rust.

## Consequences

- **What compiles grows**: stepping an associated type from its back.

## Amendment: the crate's iterator given for a `T` whose `T::IntoIter` is bound (2026-11-13, #1604)

Generic code holds a `T::IntoIter` a bound says is double-ended or
exact-sized as a JS iterator that knows its back or its length. Its caller
made one only of a parameter's own bounds, `I: DoubleEndedIterator`. So the
crate's own iterator given for a `T: IntoIterator` reached generic code as
a JS iterator that knew neither. `rev_first(Up(0, 3))`'s
`t.into_iter().rev().next()` threw `it.back is not a function`, where
native Rust gives `Some(3)`, and `len_of`'s `len()` threw
`it.len is not a function`.

- **The caller asks the bounds of `T::IntoIter`, the crate's iterator
  being its own `IntoIter`.** Where they say it's stepped from its back or
  asked its length, the iterator is made knowing them, as an `I` so bound
  is (`stepping.rs`).
- **The dictionary of that extension is the JS iterator's**:
  `{ next_back: $nextBack }` and `$foldBack`, or `{ len }`
  (`made_iterator_extension_dictionary`). It isn't the impl's, which
  would step the iterator as it is. `$foldBack` reaches the iterator's own
  `rfold` through the value and says whether that took it (#649). So std's
  compiled `Rev`, whose `fold` reaches the crate's `rfold`, doesn't drop
  the iterator again; without it, an iterator with a destructor was
  dropped twice. One entry for a JS iterator that knows
  its back or its length, `knowing_entry`, is what this dictionary and
  `iterator_extension_dictionary` both give.
- **Another's items know them as they're made**: a `Vec`'s, `$intoIter`'s.
- **A `T::IntoIter` held as it is, of a `T` another bound names**, an
  `impl Is<T>`'s (#966), is stepped from its back by its dictionaries, the
  JS iterator `given_js_iterator` makes of it, as `rev()` of it already
  was. Its `rfold`, `nth_back` and `rfind` (`back_step`) stepped the
  caller's own value as a JS iterator, and `rfold` threw
  `it[by] is not a function` on main too. The metamorphic net's
  `impl-owned` rewrite of this case found it.

`test/corpus/into_iter_bound_stepped_from_its_back.rs` holds `rev()`,
`len()`, `map(..).rev()`, `rev().count()`, `rev().last()`, `rfold` and
`rev().fold(..)` of one that writes its own `rfold` and drops, an
`impl Is<T>`'s `rfold`, and giving
`t.into_iter()` on to an `I: DoubleEndedIterator` or an
`I: ExactSizeIterator`, of the crate's iterator and of a `Vec`. It runs
against native Rust in both builds.

## Revised: a copy is its source only where it asks what its source does (2026-11-19, #2130, #2134)

std's `Copied` passes every step on to its source except `nth_back`. Its
`nth_back` is the trait's default, so it calls its source's
`advance_back_by` and then `next_back`. A `copied()` taken as its source
therefore answered `nth_back` with the crate's own.
`Refs(&items).copied().nth_back(1)` printed `own nth_back 1` where native
Rust steps `back back`, both directly and through `rev().nth(1)`, in
generic code too.

- **A `copied()` is its source in JS only where std's `Copied` reaches
  each of the crate's own steps that its source reaches**
  (`is_its_source`, `reaches_as_its_source`). That's every step but
  `nth_back`, so only a source that writes `nth_back` loses it. `is_its_source`
  is the one answer: each reader of `copied_source` for a question about
  the copy's form asks it first, and an architecture row lists the rest,
  which ask a question about the type that also holds of a map.
- **Another copy is a map of its source**: `$copied` where it reaches the
  crate's own `nth`, `last` or `count`, and `$mapEnds` or `.map` otherwise.
  Its maker, `copied_items`, makes it knowing its back wherever its type
  is stepped from it, with its source made knowing it underneath. Where it
  isn't its source, nothing past its maker takes it for its source: on
  main, a copy of `Copy` structs stepped through `rev()` threw a
  `TypeError` that way. Such a copy is refused there, and no case found
  reaches that refusal.
- **Generic code's copy of a `DoubleEndedIterator` type parameter is
  `$copiedGiven(it)`**: `it` itself unless the value its caller gives
  writes its own `nth_back`, and `$copied` of it if it does. A clone of one,
  `$givenClone`'s, is one of a clone of what it's made of. An
  `Iterator`-only parameter's copy stays its source, because nothing steps
  it from its back.
- **A type parameter bound by `DoubleEndedIterator` may reach its caller's
  own `nth_back`** (`Reach::Sure`), as with `nth`, `last` and `count`. Its
  helper, `$nthBack`, asks the value itself. Its dictionary, where it's
  given as it is, carries `nthBack: $nthBack`, or `$nthBackSome` of
  boxed items, beside `rfold: $foldBack`. So generic `by_ref().rev().nth(0)`
  and `&mut I`'s `rev().nth(1)` call the caller's own (#2134); the second
  was wrong only in the build of std's structs.
- **A copy the crate clones is made of its source's value** (`makeCopied`),
  never of the JS iterator made of it. Its clone took that iterator for the
  value, and threw. Where it's double-ended over the crate's own iterator,
  it's made knowing its back, because nothing past its maker can.

Case (ADR 0262): R. Which of the crate's steps std calls is observable.
Corpus cases `copied_hides_its_sources_own_nth_back`,
`copied_structs_stepped_from_their_back` and
`generic_nth_back_reaches_what_its_given` run as native in both builds.
Mutations: `copied-is-its-source-anyway`, `copied-given-as-it-is`,
`copied-apart-backless`, `param-nth-back-unreached`,
`copied-given-passes-nth-back` and `given-clone-of-a-copy-whole`;
`copied-made-unparted` moved to the maker's new place, and nine
retargeted. `source-own-count-unasked` went with its code: since #2112's
step 1b, `count()` asks the catalog, whose `reaches_its_own` asks
`source_writes` itself. So `repeat_n`'s own `count` was a second answer,
one that also said yes of a `by_ref()`'s, whose `count` is std's default.

**Amended (2026-11-19, #2137): a `cloned()` of what generic code is
given is made knowing its back as a `copied()` is.** Generic
`i.cloned().clone().next_back()` threw `it.back is not a function`.
The `cloned()` was cloned apart and made of `$cloned($given(it,
IIterator))`, and its source was never given its back, because only
the clone was stepped from it. The `$cloned` form and the
`$copiedGiven` form now ask one helper, `given_knowing_its_back`. It
makes what generic code was given knowing its back wherever the copy
is to know it and its `DoubleEndedIterator` evidence is given.
`copied().clone().nth_back()`, #2137's own case, already ran as native
since #2130's `$copiedGiven`.

Case (ADR 0262): R. A throw where Rust steps is a wrong answer. Corpus
case `cloned_of_a_generic_source_cloned_and_stepped_from_its_back` runs
as native in both builds. Mutation: `given-cloned-backless`.

## Amendment: a type parameter another trait's method gives is reversed as one given as it is (2026-11-19, #2139, #2147)

`rev_called` made `$rev` of a type parameter's given JS iterator, which
knows its own `nth_back` and `rfold`, only where the crate stepped the
`rev()` from its back, or where the parameter drops what it holds. Any
other use went to `reversed`, a JS iterator of the dictionary's
`next_back` alone, which knows none of the parameter's own steps. A
parameter reached as a parameter's own type was right, since its caller
gives it as a JS iterator (#2134). One reached only through another
trait's method, `Is::is(i)` or `GetMut::get_mut(i)` of `impl Is<I>`,
`&mut impl GetMut<I>` or `&mut dyn GetMut<I>`, was wrong in two ways.
Its `rev().nth(1)` stepped `next_back` twice where native calls its own
`nth_back` (#2139). By value, its `rev().fold()` stepped `next_back`
where native calls its own `rfold` (#2147). Now a type parameter given as
it is is always `$rev` of its given JS iterator, closed to drop it,
whatever is asked of it. A `&mut` to one is lent first (`$lent`), so it
passes on only what std's `&mut I` passes on: its `nth_back` its own, its
`rfold` by its `try_rfold` (ADR 0928), never closed. ioDC's first try
gave the `&mut` one unlent, and `mut_of_an_iterator_passes_on_what_std_does`
then called `R`'s own `rfold` through `by_ref().rev()`.

Case (ADR 0262): R, results change. Three snapshots move, each a
`$iterator(it, I.next_back)` becoming `$rev($given(it, ..))` (lent where
it's a `&mut`): `filter_map_of_given_from_its_back`,
`given_iterator_extensions` and `mut_of_an_iterator_passes_on_what_std_does`,
each run as native. `generic_nth_back_reaches_what_its_given` adds each
form through another trait (by value, `&mut impl` and `&mut dyn`, `nth`
and `fold`), each run as native. Its metamorphic rewrites `dyn-ref`,
`impl-ref` and `impl-owned`, which found #2139, are taken off the known
list. The readability baseline's `renamed x$1` share grows 447.6 to
447.7, all of it the added lines' `println!("{:?}", f(..))` temporaries,
`value$10` and on, as the case's existing lines already name theirs.
Mutations: `rev-of-given-only-where-stepped-back`,
`rev-of-a-lent-given-unlent` and `rev-of-a-lent-given-by-its-next-back`,
caught. `given-as-it-is-rev-unclosed` and `rev-of-given-param-unknowing`
are retargeted to the arm's new lines, and caught.

## Amendment: the dictionary's `next_back` arm goes, unreached

With a parameter given as it is always `$rev($given(..))`, lent where
it's a `&mut`, `rev_called`'s later arm, `reversed`, a JS iterator
stepped by the `DoubleEndedIterator` dictionary's `next_back`, is
reached by nothing: it asks the same dictionary `given_js_iterator`
does, so wherever it would answer, one of the two arms before it has.
A marker build found it executed by none of `check:fast --all`, the
corpus with std's iterator structs, the consumer net or the twin nets,
and its mutation, `given-rev-refused`, survived the whole corpus with
its snapshots. The arm, `reversed` and the mutation go together (ADR
0093's unreached evidence).
