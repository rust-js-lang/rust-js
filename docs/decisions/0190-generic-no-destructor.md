# 0190. Generic code may take no destructor of a type parameter, and its callers are checked

Status: Accepted. Extends [0098](0098-destructors.md) and [0100](0100-separate-crates.md).
Amended by [0464](0464-a-pick-drops-what-it-passes-over.md): a generic
iterator consumed by value, a `fold` of it, is dropped after by its
caller's drop, so its callers may give one with a destructor. And by
[0465](0465-dropping-an-adapter-is-closing-it.md): a lazy adapter of one,
closed by its caller's drop, too.

Case: D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js runs a value's destructor where Rust does (ADR 0098). A generic
function's type parameter may be given one with a destructor, so its drop
is the caller's to give, `dropT`, and what rust-js can't drop of one is an
error: a `for` loop over a generic iterator, which drops what it hasn't
reached when it leaves early, a `fold` of one, `Vec::clear` of a `Vec<T>`.

chrono's are such, its parser's and formatter's loops and its `Sum`s'
`fold`s, five places, though nothing it's given has a destructor: a
library's type parameters may be given one by a consumer.

## Decision

**What a generic function would drop of a type parameter, and can't, is
taken as nothing, and the function then takes no destructor of that
parameter. A call that gives it one is an error, at the call:**

```text
error: rust-js does not support giving `total`'s `I` a type with a destructor, where it takes none yet
 --> src/lib.rs:5:21
  |
5 | pub fn f() -> u32 { total(Counter(0)) }
```

- **Only where it would be an error:** a loop over a generic iterator, a
  std call that would drop what it's given (amended by
  [ADR 0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md):
  not one that drops none of it, `swap`, `get_mut`, `index_mut`,
  `extend_from_slice`, `set_len`, nor `take` of a `repeat`), and only where what it would
  drop is the type parameters' alone, `I` or a `Vec<T>`, not a type's of
  its own.
- **A caller that gives its own type parameter takes none of it too,**
  `relay<J>` calling `total(j)`: the crate's calls are checked once each
  function is lowered, as one may be lowered before what it calls.
- **A trait impl's method takes none of the impl's parameter where its
  dictionary is made too:** the dictionary gives each method the impl's
  parameters, a call like any other, so a call through one, `e.empty()` of
  an `E: Empty`, is checked where the caller makes `Wrap<Loud>`'s.
- **A library's are in its manifest,** `no_drops`, and its consumers'
  calls are checked as they're lowered.

## Why

- **It's exact:** nothing is dropped wrong, as nothing given has one; and
  what would be is an error, at the call that gives it, which is where a
  reader looks. The `generic_iterator_items` corpus case loops over, steps
  and folds generic iterators, as chrono does, with native Rust, and the
  diagnostics and crate tests check each call that gives one is refused:
  directly, through another generic function, through a dictionary, and
  of a library's.
- **chrono's five places compile,** and the generic code of any library
  that's given no destructors.

## Alternatives

- **Drop what's left of a generic iterator:** (Amended: a std trait's
  associated type's, an `I::IntoIter`, is its caller's drop, ADR 0421; a
  loop over a type parameter itself still takes none.) the iterator's drop, a
  caller's, of the JS iterator it's been made, and that of what it holds,
  for std's `vec::IntoIter` of them too, which rust-js can't follow (ADR
  0098). The whole design, for callers chrono doesn't have.

## Costs

- **A call Rust takes may be refused,** where it gives one with a
  destructor: a `for` loop over an iterator of them in generic code.
- **A requirement is the parameter's, not the place's:** one loop over `I`
  takes none of `I` for the whole function.

## Amended (#180)

`nth`, through a `&mut` or of an iterator it ends holding nothing of, is
one of the steps that leave no item undropped (`IterComb::leaves_none`):
it drops each item it passes over (`$nthDropping`) and gives back the one
it stops at. So a generic function given, by value, an iterator with a
destructor that it steps by `nth` takes that iterator's drop, where it was
refused.

## Amended by

[ADR 0712](0712-what-lowering-finds-of-drops-is-returned-with-it.md): what a function's lowering finds of drops is returned with it, `DropsObserved`, and gathered by the pipeline, not written into the crate's facts.

## Amendment: generic code closes its caller's iterator as a panic unwinds (2026-11-14, #1753)

Generic code's type parameters drop nothing of its own; their caller gives
the drop, here as the `$closing` generic code is handed. So a collect in
generic code of the caller's iterator, `it.collect()` of an `I`, took it as
holding nothing to drop and collected it as JS does, `Array.from`, which
never closes an iterator whose `next` throws: a panic in a step left the
caller's iterator unclosed, and its drop, a crate's `Own` iterator's, never
ran. Now an iterator that is, or holds by value, its caller's, a type
parameter's or its associated type's (`I`, `I::IntoIter`, `Map<I, F>`,
`holds_callers_iterator`), is collected as one that holds what drops
(`unwinding_collect`), closed as the panic leaves it. Not a slice's
iterator of a type parameter's items, which holds none of them. The
corpus's 53 generic collects of a caller's iterator move from
`Array.from(..)` to `$foldToArray(..)` or `$collectDropping(..)`, each
printing what native does. New corpus
`generic_collect_of_a_panicking_chain_drops_its_source`, into a `String` and
a `Vec`; mutation `callers-iterator-not-closed-on-a-panic`, caught, and
`collect-holding-unclosed` retargeted, caught.

## Amendment: a trait's method is checked as a function is (2026-11-22, #2276)

Case: D ([0262](0262-when-rust-and-js-disagree.md)): rust-js can't keep
Rust's behaviour here, since JS wouldn't drop the value, so the call is
refused at compile time.

A trait's default that passed its `I` on to a function taking no
destructor of it compiled, and the iterator's drop was lost:
`Z.via(D(7), Box::new(Own(3)))` printed no `drop Own`. The free-function
form was refused. There were three gaps:

- A call through a trait's dictionary gave its method's own evidence by
  `own_evidence`, which checked nothing. It now asks `check_drops_given`,
  as `evidence_args_given` does.
- A default copied into an impl recorded its requirement at the impl's
  numbering of its parameters. It now records them as the default numbers
  them (`declared_param`, #2273). One of the impl's own parameters is
  recorded as its trait's `Self`, which holds it, as a blanket impl's `I`
  is its `Self`, never against a sibling. itertools' `k_smallest` over its
  blanket impl stays as it compiled.
- An impl's own method's requirement was keyed by that method,
  `<Y as Via>::via`, and a call through the trait was checked against
  `Via::via`.

**A call of a trait's method rustc resolves to one method, an impl's or
the trait's default, is checked against that method's own requirement. A
call it resolves to none, generic code's or a `dyn`'s, is checked against
every impl's requirement as well.** Each impl's requirement on its own
type parameters is lifted to the trait's method's numbering
(`trait_method_param`). So `Y`'s override, which folds a box of its `I`,
refuses `through(&Y)` and generic code's `t.via(..)`, but not `Z.via(..)`
on a `Z` that counts and drops it. A requirement on an impl's own type
parameter, outside the method's, is checked where the impl's dictionary
is made, `<W<A> as Tot>`'s `A`, as before.

test/diagnostics.test.ts covers a default in impls of no, one and two type
parameters, a generic call, an override and a generic call of the
override. Corpus `trait_call_checks_the_impl_it_resolves_to_for_drops`
holds the calls that resolve to a method that drops, against native Rust.

## Amended 2026-11-25: a library's function the crate compiles itself (#2575)

A consumer compiles its own copy of a library's generic function where
its call gives more than the export takes, a type with a destructor
among them (ADR 0914). The copy's lowering records what it takes none
of, as the crate's own functions' does: a std call that takes a value
with a destructor, `char::decode_utf16(it)`, whose runtime iterator,
`$decodeUtf16`, can't drop what it holds. But a call was checked against
those requirements only where its callee was the crate's own, and a
library's export only against its manifest's `NoDrop`, which the
library writes of what its own calls observed. So the consumer's call
`dep::decoded(Loud { .. })` went unchecked, and its copy, given a drop
for `I`, dropped `iter.iter`, a field the runtime's iterator hasn't:
`TypeError` at the loop's end, where Rust drops the iterator.

**Each call of a function not std's is checked once the crate is
lowered, a library's too**: of one the crate compiles itself, its
lowering here says what it takes none of; of one it doesn't, nothing is
recorded, and the manifest's check is as before. The consumer's call is
refused with one crate's words: "giving `dep::decoded`'s `I` a type with
a destructor, where it takes none". ADR 0262's last row: Rust's drop
can't be kept until the runtime's iterator drops what it holds, so a
compile error.

test/crates-refused.test.ts holds the pair,
`library_decode_utf16_of_an_iterator_that_drops`, native-run, refused at
both calls; mutation `library-copy-drops-unchecked`.
