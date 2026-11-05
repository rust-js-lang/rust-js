# 0545. A map the crate steps from its back is made knowing it

Status: Accepted; a filter of an array's items is one too, since
[0990](0990-a-filter-of-an-arrays-items-stepped-from-its-back.md). Extends [0455](0455-stepping-from-the-back.md),
[0488](0488-an-extension-bound-iterator-is-an-iterator.md) and [0535](0535-a-lazy-map-steps-from-its-back.md);
accepts more of the [iterators draft](draft/cloned-and-buffering-iterators.md)'s
analysis. Amended: a type parameter's map or filter where its bounds give
no back is its JS iterator forward alone; a `rev()` whose own back step
the crate calls is one too (#702).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): its items from its
back, and when its function runs, are Rust's.

## Context

itertools' `PadUsing::rfold`, `(self.iter.len()..self.min)
.map(self.filler).rfold(init, &mut f)`, was refused as a library: moving
`init`, which may drop there, writes a drop flag, which first makes what's
pending, the `map` call, a variable, as a call's cleanup reads the flags
as they were where it was made (ADR 0098). A map in a variable has lost
the source its back would step: "a `Map<Range<usize>, F>` as a
`DoubleEndedIterator`". The same of `let mut squares = (1..n).map(f);
squares.next_back()`. The draft asks the iterator's owner to choose before
lowering, not from the JS a value was made of.

## Decision

**`analysis::back_stepped` is the one owner of which `map` and `filter`
types the crate steps from their back: each a `DoubleEndedIterator`
method's `Self`, `next_back`'s, `rfold`'s and the like, of a source not
an array's.** `stepped_from_its_back` is the one answer the lowering asks.

- Such a type knows its back (`knows_its_back`), so where it's made, a
  map is `$mapEnds` and a filter `$filterEnds`, of a range made knowing
  its back too; where it's stepped, it steps as it is, in a variable or
  not.
- A source given as it is gets its back where the map's made, as ADR 0488
  gives it, where its bounds say it has one; where they don't, `either`'s
  forward-only methods, it's made as before.
- `rev()` isn't among them: its own forms reverse an array's or a range's
  items eagerly where that reads better (ADR 0164). Amended: one whose own
  `rfold`, `nth_back` or `rfind` the crate calls is, below (#702).

## Why

`map_stepped_from_its_back` is `PadUsing::rfold` in a crate with a type
that may drop, a range's map a variable holds stepped from its back, and
a map of an `ExactSizeIterator` given as it is counted, a range's too,
against native Rust. `extension_map_rfold` holds a map of a type
parameter given as it is in a variable and steps it from its back.

## Consequences

- itertools compiles whole as a library: no refusal left.
- A chain's map stepped from its back is made as it was stepped before,
  `$mapEnds(..)`.
- Mutations see no such type, make it unknowing, give its range as it is,
  and leave out ADR 0488's back or a source's dictionaries.

## Amendment: a map of a `Vec`'s items with a destructor

`vec.into_iter().map(f).rev()` of items with a destructor was refused once
the map was held in a variable: an array's map is left to native forms,
which can't step a source closed to drop its items (ADR 0465), `$closing`.
Such a map, `back_stepped`'s `dropping_items`, is made knowing its back
where it's made, the type `rev()` asks too: `$mapEnds` of its source
closed to drop what it hasn't given, which `$mapEnds` closes as it's
closed, and the `Rev` of it is closed as it's dropped
(`closed_on_drop`). What it never gave is dropped front to back, as Rust
drops it. `test/corpus/rev_lazy_drops.rs` against native Rust; mutations
`rev-of-dropping-map-unasked`, `map-ends-source-unclosed`,
`map-ends-unclosed` and `rev-of-closing-unclosed`.

## Amendment: a flatten from its back

`rev()`, `next_back()` and the like of a `flatten()`, or of a `flat_map(f)`
whose `f` gives an array or a `Vec`, were refused (#165, the bug hunt). A
flatten the crate steps from its back is made knowing it where it's made,
as a map is: `$flattenEnds(src, each)`, which keeps a front and a back
inner list, steps the source from the end it's stepped from, and, the
source done, gives the other end's, as std's `FlattenCompat` does; `f` is
run as each item is reached. A `flat_map` whose `f` gives a lazy iterator
is refused still: its items would have to be made knowing their backs.
Held by `test/corpus/flatten_from_its_back.rs`.

## Amendment: what a function gives behind an `impl DoubleEndedIterator`

A map, a filter or a chain of a slice's items a function gave behind an
`impl DoubleEndedIterator` was refused where its caller stepped it from
its back (#184). The analysis left out a map or a filter of an array's
items, whose own forms step it in place, but the caller has only the
call: no form of its own steps it.

- **The analysis records each adapter a type given behind an `impl` is
  made of** (`given`, where the type asked isn't the one it reveals), an
  array's items' too, and a chain of them, from a call that asks it to be
  double-ended or of a `DoubleEndedIterator` method. A closure's type is
  its body's alone, so what's made in place elsewhere is as it was.
- **Where it's made, it's made knowing its back**: a map `$mapEnds`, a
  filter `$filterEnds`, a chain `$chainEnds`, asked by `knows_its_back`,
  which asks what an `impl` hides, and by `knows_its_length`, which knows a
  map of an array's items made so knows its length too.
- **What drops is decided of the type as it's written**: revealing a
  call's output where it's lowered dropped a consumed iterator early,
  `drops_of_consumed_iterators`'s, so only `knows_its_back` and
  `knows_its_length` reveal it.

ADR 0262's case: C, an error where Rust is fine. Held by
`test/corpus/returned_impl_ends.rs` and the mutations
`given-impl-not-back-stepped`, `given-chain-not-back-stepped`,
`knows-back-of-impl-hidden`, `mapped-array-length-unknown` and
`given-chain-unmade`.

## Amendment: what a map or a filter stepped from its back steps

A map or a filter stepped from its back whose source is another adapter
was accepted and threw `it.back is not a function` (#218):
`[1, 2, 3].iter().map(f).map(g).next_back()` was `$mapEnds` of JS's own
`.map`, which has no back; `(0..9).skip(2).map(f)`'s was `$lazyRange(2, 9)`.
`back_stepped` marked the adapter stepped alone, and `back_kept` took its
source as it was. Now:

- **What it steps is marked too**: each map or filter that a marked one's
  source chain holds, through what steps from its back where its source
  does (`back_stepped_source`), is made knowing its back where it's made,
  held in a variable or not. A `filter_map` held so has no such form; it
  stays unmarked and is refused, not stepped as a JS iterator with no
  back.
- **A source made here is made knowing its back where it's stepped**:
  `back_kept`'s map and filter of a source without a back, a `skip`, a
  `take`, a `step_by`, an `enumerate`, a `chain` or a `fuse` made in
  place, give it to `as_double_ended`, which makes it knowing its back or
  refuses it.

ADR 0262's case: C, an error where Rust is fine, of what isn't made yet;
nothing is approximated. Of 1,100 generated programs pairing adapters
under a back-stepping consumer, none throws: 32 more agree and 44 are
refused, an adapter of the crate's own iterator that steps from its back
(#184). Held by `test/corpus/back_stepped_adapter_sources.rs` and
`held_filter_map_back_refused.rs`, and five mutations. The readability
baseline's `$iter` share grows by one in 10,000 lines: the corpus case's
thirteen arrays stepped from their back are each `$iter(v)`, the source
`$mapEnds` and `$filterEnds` step from either end.

## Amendment: a type parameter's map where its bounds give no back

The amendment above refused the `either` crate, which the guide's
crates.io page compiles: its `IterEither::fold` is `inner.map(Right)
.fold(init, f)` of an `R: Iterator`, and its `rfold`, of an `R:
DoubleEndedIterator`, steps the same `Map<R, _>` from its back. The type
being stepped from its back, `fold`'s map asked its source's back, which
an `Iterator` bound doesn't give: "a `R` as a `DoubleEndedIterator`".
Given as it was before, the range a caller gives such an `I` was no JS
iterator, `it.next is not a function`.

**A map's or a filter's source of a type parameter whose bounds give no
back is its JS iterator forward alone, `$given(it, IIterator)`**
(`backs.rs`, `given_stepped`): a body steps a type from its back only
where a `DoubleEndedIterator` bound says it can, which rustc asks, and
that body's source is given its back.

Held by `test/corpus/generic_map_folded_and_rfolded.rs`, a wrapper's
`fold` and `rfold`, and a filter's `count` and `next_back`, of a range and
of a forward-only `repeat(..).take(..)`, against native Rust; and the
mutation `type-parameter-source-unstepped` (`lower/mir/iter/backs.ts`).

## Amendment: an `inspect(f)` too

`Down { n: 3 }.inspect(|x| seen.push(*x)).rev()` and
`(1..5).inspect(f).next_back()` were refused. An `inspect(f)` is made as
a map, `src.map((item) => { f(item); return item; })` (ADR 0136), so
it's stepped from its back as a map is: `$mapEnds` of its source made
knowing its back, `f` called on each item as Rust's `Inspect` gives it.

- `as_double_ended` and `as_exact_size` read an `Inspect` made in place
  as a map. `as_double_ended`'s rule of a map made here comes before its
  "already knows its back" answer: a `.map(f)` form returned as it is
  has no back, whatever its type says.
- The back-stepped analysis records an `Inspect`, as it does a map, and
  it's made knowing its back where it's made, as a flatten is.
- `test/corpus/inspect_from_back.rs` checks the crate's own iterator, a
  `Vec`'s and a range's against native Rust, the order `f` sees items
  in too.
- The mutations are `inspect-made-here-backless` and
  `inspect-back-stepped-unmade`.

## Amendment (2026-11-02): a `filter_map(f)` too (#473)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; valid Rust was
refused. itertools' `FilterMapOk::rfold`, `self.iter.filter_map(|v|
transpose_result(v.map(&mut f))).rfold(init, fold_f)`, of an `I:
DoubleEndedIterator` a struct holds as it is, was refused, "a
`FilterMap<I, ..>` as a `DoubleEndedIterator`, which rust-js can't step
from its back". A `filter_map` of a source that knows its back was made
knowing it, `$filterEnds($mapEnds(..))` (ADR 0526); of a source given as
it is, it was made `src.map(f).filter(present)`, with no back.
453619635 took away the line that said such a map knew its back
(`made_knowing`), which said so of a value made without it: put back, its
fold threw, `it.back is not a function`. A refusal was right; this is the
form.

**A `filter_map(f)` is stepped from its back as a map filtered: `$filterEnds`
of `$mapEnds` of its source made knowing its back, `$mapEnds(..,
$someValue)` after it where a kept value looks like `None`.**

- `as_double_ended` reads a `filter_map` made in place, `.map(f).filter(present)`,
  each kept `Some` unboxed after, `.map($someValue)`, before its "already
  knows its back" answer, as it reads a map's.
- The back-stepped analysis records a `FilterMap` as it does a filter, and
  it's made knowing its back where it's made, as an `inspect(f)` is: a
  library's, held in a variable before it's folded, too.
- `test/corpus/filter_map_of_given_from_its_back.rs` checks a struct's
  `rfold` of a held `I`'s `filter_map`, and one whose kept values look like
  `None`, against native Rust, as a program and as a library; the crate
  corpus's itertools compiles. `held_filter_map_back_refused.rs`, a
  `filter_map` held in a variable that a held map steps from its back,
  refused since #218 (and in ADR 0871), runs now, against native Rust, as
  `held_filter_map_from_its_back.rs`.
- The mutations are `filter-map-ends-unmade`, `given-filter-map-ends-boxed`
  (`lower/iterators/knowing.ts`), `filter-map-unstepped-from-back`
  (`lower/analysis/back_stepped.ts`) and `filter-map-made-unknowing`
  (`lower/mir/std_calls.ts`).

## Amendment (2026-11-02): one held, given to a `rev()` or generic code (#484)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; valid Rust was
refused. `let kept = v.into_iter().filter_map(Some); kept.rev()` was
refused, "a `FilterMap<..>` as a `DoubleEndedIterator`, which rust-js
can't step from its back", and so was a held `map`'s or `filter`'s
`rev()`, `let more = v.iter().map(f); more.rev()`, and a held adapter
given to a function bound by `DoubleEndedIterator`, `last(doubled)`. The
`rev()` was made in place, so `held_rev` didn't hold; the type it asked
was its argument's own, `held == asked`; so nothing marked the adapter,
and the variable held it made without its back, which no form of
`rev()`'s reads. `next_back()` of the same variable ran: the analysis
marks what `DoubleEndedIterator`'s own methods are called on.

**What a call asks to be double-ended, given it held, a variable's, a
field's or a place's, not made where it's given, is made knowing its back
where it's made, as one given behind an `impl` is.**

- The back-stepped analysis reads each call's arguments: one of the type
  a clause asks `DoubleEndedIterator` of, which isn't, through its
  borrows, what a call gives, is held. `it.by_ref().rev()`'s is
  `by_ref()`'s, std's struct `it` holds, stepped by its dictionaries as
  before. Not of `ExactSizeIterator` alone, and not of
  `DoubleEndedIterator`'s own methods, whose adapters the analysis's
  second rule decides of, of its own form where it has one.
- One made in place, `v.iter().filter(p).rev()`, keeps its form,
  `v.toReversed().filter(p)`.
- A call by its path, `last(doubled)`, is found as its callee's, its
  `fun` stripped of its scope; a `rev()` called so is too.
- `test/corpus/held_filter_map_reversed.rs` checks a held `filter_map`
  of `Some`, of a closure and of a function, reversed and stepped; a
  held map of an array's items and a filter of a range, reversed; a held
  `filter_map` given to generic code that steps it from its back; and one
  made where it's reversed, against native Rust.
- The mutations are `held-given-unrecorded`, `held-given-of-items-native`,
  `made-in-place-taken-held`, `held-through-borrows-unseen` and
  `path-call-uncalled` (`lower/analysis/back_stepped.ts`).

## Amendment (2026-11-04): a `rev()` stepped from its back (#702)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; valid Rust was
refused. `Folds(0, 2).map(|d| vec![D(d.0 * 10), d]).flatten().rev()
.rfold(String::new(), f)`, of the crate's iterator `Folds` whose items `D`
have a destructor, was refused: "a `Rev<Flatten<Map<Folds, ..>>>` as a
`DoubleEndedIterator`, which rust-js can't step from its back". Of `u32`s
it compiled. The cause wasn't the crate's iterator, nor what std's
adapters ask of it (ADR 1089's `OwnStep`, which says `Rev::rfold` is its
source's `fold`), but where the `rev()` is when it's stepped: MIR holds
it in a variable before its `rfold` where a call between may unwind and
drop it, `String::new()`'s, so a `Rev` of what has a destructor is a
`const` by then, and `nth_back(n)` takes it by `&mut`, so
`v.iter().map(f).rev().nth_back(1)` of numbers was refused too, and
`(0..5).rev().nth_back(mk())`, `Folds(0, 3).rev().nth_back(mk())` and a
`Vec`'s map's or filter's `rev().rfold(String::new(), f)`. Each step read
the `rev()` by its form (ADR 0987), which a variable doesn't have: the
first decision above left `rev()` out, its forms reading better.

**A `rev()` whose own `DoubleEndedIterator` method the crate calls,
`rfold`, `nth_back` or `rfind`, is recorded by `back_stepped`, as a
flatten is, and made knowing its back where it's made.**

- `knowing::reversed_made_knowing` is the one answer, asked where it's
  made, `mir_iter_method`'s `rev()`, and where it's stepped,
  `as_double_ended`, which takes one made in place as it is, held or not.
  Not of a type parameter's or a zip's, which `reversed` steps by what
  it's given, as before. Not `knows_its_back`, which answers of a type:
  std's struct of the same type, a `Rev` a `Vec` or a field holds (ADR
  0871), is given its back where it's stepped, `knowing_held`, as before,
  so a type answer would take it as it is, with no back.
- A flatten a `rev()` or generic code asks to be double-ended is recorded
  too, wherever it's made: its forms never step from the back, and
  `as_double_ended` made it `$flattenEnds` where it was asked, by its form,
  which MIR may hold in a variable first, where the call between has a
  cleanup that drops what's live beside it, a `found` of an earlier
  statement. A crate's two flattens of different types were refused
  together where each compiled alone. It's made `$flattenEnds` where it's
  made now, so its JS is as it was. Not one whose items are std's adapters,
  `groups.iter().flat_map(|g| g.iter().map(f))`: made knowing where it's
  made, the closure's JS `.map` is taken for std's struct by the outer
  call's span (`compiled_struct_at`) and steps nothing, as that flatten's
  `next_back()` does already on main, a wrong answer filed as #716; at its
  `rev()` it's made as before (`flatten_of_known_items`).
- Where it's made, `reversed_knowing` makes it so: the crate's iterator's
  by its own `next` (`reversed_user_knowing`), another by its form
  (`reversal_knowing`, the reader `as_double_ended` had): `$rev(it)` and an
  array's items reversed as they are, a range stepped from its end
  `$rev` of the range knowing its back, a chain over an array's items
  reversed its source's, `$mapEnds($iter(v.toReversed()), f)`. Any other
  form is refused there, never stepped as a JS iterator with no back.
- What it reads is as it was where the form was readable: the consumer
  made the same form knowing its back then, the step later; a range's
  `rev()` the crate steps from its back elsewhere in the crate, of the
  same type, steps `$rev($iterator(range, $rangeNext, false,
  $rangeNextBack))` where it was `$iterator(range, $rangeNextBack)`.
- **The user's decision (2026-11-05): per type.** A `rev()` the crate
  steps from its back anywhere is made knowing its back for every value of
  its type, as a map is, so one only ever stepped forward reads the longer
  form too: `$iterator([3], countdownDoubleEndedIterator_nextBack, false,
  countdownIterator_next)` where it was `$iterator([3],
  countdownDoubleEndedIterator_nextBack)`. The alternative, a fact of each
  `rev()` call stepped from its back, reads tighter where a type's `rev()`s
  differ, but needs a fact carried from each step back to where its value
  was made through every place MIR may hold it, the bookkeeping this
  amendment's flatten needs already; one type, one form, is the general
  rule `back_stepped` answers.
- Its order is std's: a `rev()`'s `rfold` folds its source forward, its
  flatten's `next` of each inner, `$rev`'s `fold` its source's, and what
  `Folds::next` panics part-way drops the accumulator, as std unwinds.
- Still refused (#718, P2): a lazy `rev()`'s `next_back()`, a type parameter's
  `rev().nth_back(n)`, a zip's, a `chain()` of a `Vec`'s items with a
  destructor reversed, and a map of a range reversed held so
  (`(0..5).map(f).rev().rfold(mk(), g)`), whose form isn't read.
- `test/corpus/own_iterator_flattened_from_its_back.rs` checks the issue's
  `rev().rfold`, a `flat_map`'s, a `Vec`'s, the crate's iterators
  flattened, a flatten's `rfold` and `rev().fold`, `rev().nth_back` and
  `rev().rfind`, the crate's iterator's, a slice's map's and a range's
  `rev().nth_back`, a type parameter's `rev().rfold`, std's struct of the
  crate's `rev()` a `Vec` holds, flattened, flattens beside what's live,
  `rev().rfind` and `next_back()` before a `rev().rfold` and a `rev().fold`,
  and two panics, against native Rust, as the crate builds it and with
  std's iterator structs. Its `rfind` and `next_back()` leave a flatten's
  inner part-way, whose items its close drops since #705. The mutations are `rev-stepped-from-back-unrecorded`,
  `flatten-asked-unrecorded` and `flatten-of-adapters-recorded`
  (`lower/analysis/back_stepped.ts`), `rev-made-knowing-read-by-form`,
  `rev-made-knowing-of-a-held-struct` and
  `rev-made-knowing-of-own-unknowing` (`lower/iterators/knowing.ts`) and
  `rev-made-unknowing` (`lower/mir/iter.ts`). No program found steps a
  type parameter's or a zip's `rev()` from its back that compiles either
  way, so the guard that leaves them out has no mutation: it keeps their
  forms as they were.

## Amendment: an enumeration of std's struct is recorded too (2026-11-05, #712)

`back_stepped` records an `Enumerate` of std's struct compiled wherever
it's made, a `Fuse`'s or a `Peekable`'s
(`recognition::enumerates_a_compiled_struct`), wherever it's stepped from
its back, of whatever items: by a `rev()` made in place, a bound generic
code asks, or its own `DoubleEndedIterator` methods. No form of such an
enumeration steps from its back, as it's `$enumerate` of the struct, whose
`count` is its source's; a recorded one is made knowing its back and
length where it's made. [ADR 0465](0465-dropping-an-adapter-is-closing-it.md)'s
#712 amendment says why.
