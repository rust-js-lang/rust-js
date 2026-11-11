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
- A range's own `count()`, `last()` and `nth()` of its `rev()` read its
  bounds, `.of` of what it's given: of one made knowing its back,
  `$rev($iterator(range, $rangeNext, false, $rangeNextBack))`, `$rev`'s
  `of`, its source's, as `$iterator(range, $rangeNextBack)`'s was. It had
  none, so `(1..6u32).rev().count()` threw in a crate that `rfold`s a
  range's `rev()` anywhere (#721, P0, a regression of this amendment;
  `test/corpus/rev_range_counted_beside_its_rfold.rs`, mutation
  `rev-of-unforwarded`).
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

## Amendment: a type parameter's `rev()`, a range's map's and a chain of droppers made knowing their backs (2026-11-05, #718)

#702 left out three shapes MIR holds in a variable before their back step,
each refused as there was no form left to read:

- **A type parameter's `rev()`**, `it.rev().nth_back(1)` in generic code,
  which `nth_back`'s `&mut` puts in a variable. `reversed_made_knowing`
  no longer leaves type parameters out. Of one given a JS iterator (ADR
  0061), such a `rev()` is `$rev` of it made knowing its back and closed
  to drop it (`rev_items`). `$rev` passes its source's own `nth`, `fold`
  and `try_fold` on as its `nth_back`, `rfold` and `try_rfold`, as std's
  `Rev` does, so the crate's own `nth` is still called. Of one given as
  it is (ADR 0411), which a struct holds, `reversed` steps it by its
  dictionary, a form `reversed_knowing` can't make knowing its back, so
  it's refused there, as it was before. Made knowing it by `$given`, it
  would be closed by what `$iterator` wraps, `it.of`, which `$given` of a
  `Vec`'s items doesn't make. The guide's limitations list it.
- **A range's map or filter reversed**, `(0..5).map(f).rev().rfold(mk(),
  g)`, held past `mk()`'s call. `rev_items` reverses such a chain inward,
  `$iterator(range, $rangeNextBack).map(f)`, a form `reversal` can't
  read. Where the `rev()` is made knowing its back, a form that `reversal`
  can't read isn't taken. What steps it from its back is taken instead,
  `$rev($mapEnds(..))`.
- **A chain of items with a destructor** that a `rev()` asks to be
  double-ended, which MIR holds in a variable to drop them. `back_stepped`
  records it as it records a flatten. Where it's made it's `$chainEnds`
  (`std_calls`' `back_made`), so the `rev()` steps it as it is. A chain
  given behind an `impl` is std's `Chain`, as before.

A zip's `rev()` is still left out: `$zipBack` steps only from the back,
and a zip that knows both ends is std's `Zip`, which isn't held where it's
made in place.

No other corpus snapshot changes. `rev_made_knowing_its_back.rs` checks
generic `nth_back`, `next_back` and `next`, `rfold` and `rfind` of a
`Vec`'s items and of the crate's iterator with its own `nth`, numbers, a
panicking drop, a range's map's and an inclusive range's filter's `rfold`
given a call's value, and a chain of droppers' `rev().fold`, `rev().rfold`
and a panic in its closure, against native Rust in both builds.
A flatten of a slice's maps held and stepped by `next_back()` is there
too. Mutations: `rev-of-a-parameter-not-made-knowing`
(`lower/iterators/knowing.ts`),
`rev-reversed-inward-unread` (`lower/mir/iter/backs.ts`),
`chain-of-droppers-asked-unrecorded` (`lower/analysis/back_stepped.ts`),
`chain-stepped-from-back-made-unknowing` (`lower/mir/std_calls.ts`);
`rev-chain-refused` names its line as it now reads.

## Amendment (2026-11-05): what a `flat_map()`'s closure gives (#716)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change.
`groups.iter().flat_map(|g| g.iter().map(|x| x * 10)).next_back()` gave
`None`, where Rust gives `Some(50)`. The flatten, stepped from its back,
is made knowing its back where it's made, `$flattenEnds`, and what its
closure gives is made knowing its back too. The closure's `map()` is made
in place: a closure `flat_map` is given makes what it gives in place, as
what's stepped where it's given (`body_queries::results_in_place`). But
`knowing_held` judged it by the flatten call's span, which isn't the
closure body's, so it wasn't among that body's adapters made in place. It
took the `map()` for std's compiled `Map` and gave the JS `.map()` chain
`mapDoubleEndedIterator_nextBack`, which stepped nothing.

**What a `flat_map()`'s closure gives is marked made in place where the
flatten made here reads it** (`as_double_ended`'s flatten), its own fact,
so its `map()` is read by its form, `$mapEnds($iter(g), f)`. A `rev()`'s
was right already: its span is among the adapters made in place.

`test/corpus/flat_map_of_maps_from_its_back.rs` checks the issue's
`next_back()`, a `rev().collect()`, an `rfold`, an `nth_back`, an `rfind`,
and closures that give an `enumerate().map()` and a `rev().map()`, against
native Rust, as the crate builds it and with std's iterator structs. The
mutation is `flat-map-closure-result-by-span`
(`lower/iterators/knowing.ts`).

A `flat_map()` held in a variable is std's compiled `FlatMap`, which
steps what its closure gives as std's struct, from its back too: its
closure's result is made in place only where the `flat_map()` is
(`results_in_place` asks the call's body's adapters made in place), where
it was of every one's, so a held one's `next_back()` threw, `it.back` of a
JS `.map()` chain (#726; `test/corpus/held_flat_map_of_maps_from_its_back.rs`,
mutation `held-flat-map-results-in-place`, `lower/body_queries.ts`).

Still apart: a `map(|g| g.iter().map(f)).flatten()`
stepped from its back is refused, and the flattens of std's adapters that
`back_stepped` leaves out (#702's amendment) stay as they were (#718).

## Amendment (2026-11-05): what a flatten's function strands as it panics (#744)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change.
`vec![D(1), D(2), D(3)].into_iter().flat_map(f).rfold(Acc(0), g)`, whose
`f` panics as it makes its second inner, leaked the accumulator: `drop acc
16` was never printed, where Rust drops it as the panic unwinds, then what
the source has left. `$flattenEnds`' fold and `rfold` called the function
that makes each inner, `each`, unstaged, where `$Steps`' `flatMap` and
`$flatMap` stage it. Its `folding` now calls it through `$staged`, which
drops what the fold holds by its `unwind` where `each` throws, as std's
fold closure drops its accumulator. The forward fold of the same shape was
right already, `$flatMap`'s; both ends of `$flattenEnds` share the fix.
`test/corpus/flatten_fold_closure_panics.rs` checks an `rfold`, a `fold`
and an `rfold` after a `next_back()` against native Rust in both builds;
the mutation is `flatten-ends-made-unstaged` (`runtime/back_steps.ts`).

## Amendment: a flatten a `rev()` asks is made in place up to it (2026-11-05, #718)

#702 made a flatten a `rev()` asks to be double-ended knowing its back
where it's made, as MIR held it in a variable before its `rev()`, past a
call whose cleanup seemed to differ, and its form was gone; but not of
items that are std's adapters a closure gives (`flatten_of_known_items`),
whose JS chain was taken for std's compiled `Map`. Both are redundant
now. #716 reads a `flat_map()`'s closure's chain by its own fact, and
#723 makes calls that unwind to the same drops share one cleanup, so the
flatten stays made in place up to its `rev()` and `as_double_ended`
makes it `$flattenEnds` there, as before. Their mutations,
`flatten-of-adapters-recorded` and `flatten-asked-unrecorded`, both
survived on main, and both rules are gone with them. A flatten inside
what's asked, a map's source, say, is still recorded with the rest
(`back_stepped_adapter`). The corpus, as the crate builds it and with
std's iterator structs, runs as before; no snapshot changed.

## Amendment: one runtime `Chain`, which knows its back where both sides do (2026-11-05, #739, #752, the user's decision)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

A chain the crate steps from its back anywhere is made knowing its back
wherever its type is made (#702, #718). It was `$chainEnds(a, b)`, a second
runtime `Chain` beside `$lazyChain(a, b)`, and it had std's back steps alone:
`rfold`, `try_rfold` and `nth_back`. So its front steps were `$Steps`'
defaults. With `back()` in the crate stepping `Own(2).chain(vec![D(7)])`
from its back, `Own(2).chain(vec![D(7)]).fold(..)`, `count()` and
`for_each(..)` elsewhere never ran the crate's own `fold` or `count`, and
the order of drops changed in `last` and `collect` (#739). Its `nth_back`
stepped what it skipped of `b` by `back()` and dropped none of it, where
std's `Chain::nth_back` is `b.advance_back_by(n)`, then `a.nth_back`
(`core/src/iter/adapters/chain.rs`): `vec![D(1), D(2)].into_iter()
.chain(vec![D(6), D(7)]).nth_back(1)` leaked `D(7)` (#752). Its `rfold`
left `a` unclosed where `b`'s fold panicked. And its sides, done, were let
go unclosed: folded by value with a closure that drops, `$consume` holds
what it folds, so nothing closes itself at its end, and the crate's own
iterator on side `a` was never dropped (#757, a regression of 1c23afa16).

The user chose one `Chain` (2026-11-05). `$chainEnds` is gone, and
`$lazyChain(a, b)` gains std's back steps where both sides know their back
(`back`, `rfold`, `try_rfold`, `advance_back_by` and `nth_back`): `b`'s,
then `a`'s, `b` let go and closed once it's done from the back, as
`and_then_or_clear` drops it, `a` stepped on there unless it's held, as
`b` is from the front (#243). Its `rfold` is `$consumed`, as its `fold`
is, and gives `ends` to `a`'s fold alone, as `Chain::rfold` gives `a` its
`f` (#734). Its `nth_back` and `advance_back_by` advance each side by its
own `advance_back_by`, a `Vec`'s one slice, else each `next_back` dropped by
the drop they're given. Each fold closes each side explicitly once it's
done with it, `fold` and `rfold` alike, and sets none aside unclosed.
knowing.rs makes a back-stepped chain `$lazyChain(a', b')` of its sides
made knowing, where it made `$chainEnds(a', b')`; that helper's name is
the only change of form. And `back_stepped`'s sources of what's stepped
from its back take in a chain, as they take a map or a filter: a map of a
chain of the crate's iterators, folded from its back, was refused where
MIR held the chain in a variable before the map (`a Chain<It, It> as a
DoubleEndedIterator`); its sides are made knowing where it's made now.

Kind: duplicated authority (two runtime `Chain`s, one with std's front
steps, one with its back steps). Area: iterators. Old: since a chain was
first made knowing its back by type (#702, #718).

`chain_stepped_from_its_back_elsewhere.rs` pairs a function that steps a
chain from its back with others that fold, count, `for_each`, `last`,
`nth`, `any` and collect a chain of its type from its front, and checks
`nth_back` past each side, an `rfold` whose `f` panics, #757's `fold` and
`rfold` by a closure that drops, and a holding map's `rfold` of a chain of
the crate's iterators, against native Rust in both builds. `test/architecture-iterators.test.ts` refuses a
second runtime `Chain`. `$chainEnds`' mutations went with its code; those
of its back steps are `lazy_chain.ts`' now.

## Amendment: an rfold's source is made with its back (2026-11-05, #778)

Two adapters the crate folds from their back, `back_stepped`'s already,
were made where they're made of a source with no back, so the fold threw
`it.back is not a function` where Rust folds (results change, ADR 0262):

- An enumeration of a map, recorded for its items' destructor, of JS's
  own `.map(f)`: `ds().into_iter().map(f).enumerate().rfold(..)`.
  `back_kept`'s enumerate arm makes a map's source as `as_exact_size`
  does, `$enumerate($mapEnds(..))`, as it did a `take()`'s and a
  `step_by()`'s (#651).
- A `copied()` that's its source, of std's compiled `fuse()` or
  `peekable()`: `nums.iter().fuse().copied().rfold(..)` was the struct
  given its dictionary's `next` alone. `back_kept`'s copy arm makes it
  knowing its back (`as_double_ended`, `knowing_held`) of std's compiled
  struct as of an adapter.

36 rows of the iterator matrix listed under #646 and #650 ran as Rust
does after that, taken off its list, most since #723.

Then the rest of #778's, by one rule in place of a list: an enumeration
`back_stepped` records, made of a source whose JS knows no back
(`!has_a_back`), is `$enumerate` of that source made knowing its length
and its back (`as_exact_size`), whatever kind it is, where `back_kept`
named a `take()`, a `step_by()` and a map alone.
`as_exact_size` reads two more forms for it: a `cloned()` of what isn't a
number, JS's `src.map(clone)`, as a map; and a `skip(n)` of items with a
destructor, `$skipping(src, n, drop)`, its source made knowing them. So
a slice's `cloned().enumerate()`, and the crate's iterator's
`skip(1).enumerate()` and `rev().enumerate()` of what has a destructor,
fold from their back as Rust's do, and #778's last 11 rows are off the
list. Asked its back, a type parameter bound by `ExactSizeIterator`
alone is made knowing its length as before (`enumerate_of_known_length`):
a back flag by the source's `DoubleEndedIterator` survived as an
equivalent. `back_folds_of_sources_made_knowing_their_back` holds each
against native Rust in both builds; mutations `enumerate-of-take-from-back-unknowing`
(its line the rule's now) and `copy-of-compiled-struct-backless`
(`lower/mir/iter/backs.ts`), and
`dropping-skip-uncounted` and `cloned-map-uncounted`
(`lower/iterators/knowing.ts`). An enumeration not recorded, of items
with nothing to drop, stepped by its own `next_back()`, is still #764's.

## Amended (2026-11-05, #751): an inspect of an array's items with a destructor too

`ds().into_iter().inspect(f).rfind(p)` of items with a destructor, and
its `next_back()`, were refused in the default build, "a
`std::iter::Inspect<std::vec::IntoIter<D>, ..>` as a
`DoubleEndedIterator`": the recognizers make an `inspect(f)` as a map,
and `back_stepped_adapter` read it as one, but `dropping_items` asked
only a map's and a filter's source, so an inspect of an array's items
whose drop runs looked like a map of an array's, which steps from its
back by its own form. Its items are closed to drop them and held in a
variable (ADR 0465), which no form steps from its back, so it was never
made knowing its back, and the `rfind` of the variable refused it.
**`dropping_items` asks an inspect's source too**, as each other reading
of this analysis takes an inspect for a map: made knowing its back where
it's made, `$mapEnds`, as a map of the same items is.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted, each item seen and dropped where std's
`Inspect` gives or passes it.

Held by corpus `inspect_of_dropping_items_from_its_back`, against native
Rust in both builds: `rfind`, `next_back` then `next`, a `rev()` of it
and an inspect of a reversed source. Mutation
`inspect-of-dropping-items-not-back-stepped` (`analysis/back_stepped.ts`).
An inspect or a map whose closure has a destructor, over a `Vec`'s
`u32`s, is refused the same way, `closure_drops`'s map of that closure
held where no form steps it from its back: its own gap, #780.

## Amendment: a chain's sides are made knowing their back where they're made (2026-11-05, #718)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, since this was a
refusal. Rust's behavior is kept.

`v.into_iter().filter(p).chain(w)` of a `Vec<D>`, stepped from its back by
`rev()` or `rfold`, was refused in the default build: "a `Filter<IntoIter<D>,
..>` as a `DoubleEndedIterator`". That was so whether the chain was held in
a variable or made in place, through `.rev().map(f).sum()` too. The same
was true of a `map()`, `inspect()` or `filter_map()` side. MIR holds side `a`
in a variable before the chain is made, where `w`'s drop flag is cleared. So
when the one runtime `Chain` is made knowing its back (`$lazyChain(a', b')`,
#739), `a` has no form left to read. `back_stepped`'s walk skips a chain's
subtree, so the side was never recorded stepped from its back.

`with_stepped_sources` now takes in a chain's two sides, as it takes in a
map's or a filter's source. Each map, filter, `filter_map`, `inspect`,
copy or chain among them, and their own sources in turn, is made knowing its
back where it's made: `$filterEnds(..)`, `$mapEnds(..)`. Whatever steps the
chain then finds each side knowing it. No other corpus snapshot changes.

Not done here, and left to #718: a `skip(n)` or `take(n)` side, and a held
`v.into_iter().skip(1)` before its `rev()` with no chain, which are refused
as before.

Kind: missing case. Area: iterators. Old: f8ea97dc3^ refuses the same
shapes. `chain_of_adapters_stepped_from_its_back.rs` checks a held and an
in-place chain whose sides are a filter, a map, an `inspect`, a
`filter_map` and a map on `b`. It steps them through a `for`, `next`, a sum,
`rfold`, `nth_back` and a collect, and once with a predicate that panics part
way. It runs against native Rust in both builds. Mutations:
`chain-sides-unstepped`, `chain-side-a-unstepped`;
`stepped-sources-one-deep` names its line as it now reads.

## Amended (2026-11-06, #780): an inspect of a dropping closure, and a map of a held `IntoIter`

Two refusals of a `Vec`'s numbers mapped or inspected by a `move` closure,
stepped from its back, in the default build, "a `Map<IntoIter<u32>, ..>`
as a `DoubleEndedIterator`":

- **An `inspect(f)` whose closure owns a value with a destructor**, `move
  |d| { let _ = &h; .. }`: `closure_drops` asked a map's `F` alone, so the
  inspect, held in a variable to drop its function (ADR 0991), looked like
  one its own form steps from its back, and was never made knowing it.
  It asks an inspect's `F` too, its second type as `Map`'s is, as each
  other reading of this analysis takes an inspect for a map (#751).
- **A map of a `move` closure that copies a field of a value with a
  destructor**, `move |d| d + g.0`: the closure owns no destructor, so the
  map steps from its back by its own form, `iter_locals.maps`' map of an
  array's items, made again where `next_back` or `rfind` steps it. But the
  field's read comes after the `Vec`'s `IntoIter` is made, so MIR's
  pending `$iter(..)` is written to a variable first, and the map was
  `it.map(f)`, which `is_items_map` doesn't read. **A map of a variable
  of a type that steps an array is recorded too** (`mapped_var`): made
  again there, it names the same variable, so no array is made twice and
  nothing is moved.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted. Held by corpus
`move_closure_adapter_from_its_back`, against native Rust in both builds:
`rfind`, `next_back` then `next` of one held, and a `rev()`. Mutations
`inspect-of-a-dropping-closure-not-back-stepped`
(`analysis/back_stepped.ts`) and `map-of-held-items-unrecorded`
(`mir/assign.ts`). The map's variable is left declared where nothing reads
it, `const map = value.map(f)`, as it is of a map of an array's items held
already: a readability gap of its own.

## Amended (2026-11-06, #792): a zip's sides, and a `cloned()` of what's cloned by code

A zip of two `cloned()`s whose `Clone` runs code, `a.iter().cloned().zip(
b.iter().cloned())`, was refused in the default build, even forward where
another use of its type steps it from its back, "a `Cloned<Iter<D>>` as an
`ExactSizeIterator`": the zip is `$trustedZipEnds` of its sides made
knowing their lengths (#740), and MIR holds a side, `x.map(clone)`, in a
variable before the zip is made, so no form of it was left to read, and
no rule made it knowing where it was made. **A recorded zip's sides are
recorded too, as a chain's are** (`with_stepped_sources`, #718), **and a
`cloned()` is an adapter that steps from its back and knows its length
where its source does**, as a `copied()` is (`back_stepped_source`,
`length_kept_source`): `copied_items` makes it `$mapEnds($iter(a), clone)`
where it's made, which knows both.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted. Held by corpus
`zip_of_cloned_items_whose_clone_runs`, against native Rust in both builds,
with a `Clone` that prints: forward, `len()`, `next_back()` and `rev()`.
No other snapshot changes. Mutations `cloned-not-back-stepped`,
`cloned-length-not-kept` (`recognition/iterators.ts`),
`zip-sides-not-back-stepped`, `cloned-source-not-recorded`
(`analysis/back_stepped.ts`).

## Amended (2026-11-06, #805): the held map made again leaves no statement

Where `next_back` or `rfind` makes a held map again, `iter_locals.maps`'
(#780), the variable that held it stayed declared, read by nothing, `const
map = value.map(f)` before `$findBack($mapEnds(value, f), ..)`. Tidy's
`unread_maps` dropped an unread `$iter(v).map(f)` alone; it can't drop any
unread `x.map(f)`, as a `collect()`'s is an array's eager `map`, which runs
its closure though nothing reads what it made. **MIR's map of a JS
iterator says it is one, `Known::Iterator`** (value.rs's `lazily_made`,
of what `method_source` made one), **and `unread_maps` drops an unread map that
says so** (`iterators::lazily_mapped`), decided by that fact, never by its
call's shape: its `const`, or MIR's `let` and the assignment a call's `try`
holds where that's its only mention, `let _5; try { _5 = _6.map(f); }`.
Making it runs nothing, a JS iterator's lazy `map` (#805's question:
`as_js_iterator` makes a std iterator's value a JS iterator, never an
array).

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; the JS
reads as a person writes it. Held by corpus
`unread_map_of_a_held_iterator`, against native Rust in both builds: a held
map's and a borrowed one's `rfind`, their closures run only as `rfind`
steps them, and a `collect()` of a map nothing reads after, whose closure
still runs on each item. `move_closure_adapter_from_its_back` loses its
unread `const value` and `const map`. Mutations `unread-map-by-its-shape`
(`iterators.ts`), `map-of-an-iterator-unmarked` (`iterators/value.ts`),
`unread-map-assignment-kept` (`mir/tidy/values.ts`).

## Amendment: a `skip(n)` or a `take(n)` held, and what's held beneath what's made in place (2026-11-05, #718, #790)

Case ([0262](0262-when-rust-and-js-disagree.md)): none for the refusals,
valid Rust that was refused; results change for what `$taking` and
`$skipping` let go undropped, so Rust's behavior is kept.

In the default build, a `Vec<D>`'s `into_iter().skip(1).rev()` was refused,
"a `Skip<IntoIter<D>>` as a `DoubleEndedIterator`", made in place or held in
a variable, alone or as a chain's side, and so was a `take(n)`'s, "as an
`ExactSizeIterator`". Of numbers, a held `skip(1)` or `take(3)` given to
`rev()` was refused too. Three causes:

- `as_double_ended`'s counted arm read a `skip(n)` made in place only as
  JS's `src.drop(n)`, never as a skip of items with a destructor,
  `$skipping(src, n, drop)`; `as_exact_size` read that one apart, and refused
  a `take(n)` of such items. **One reader, `counted_parts`, reads all three
  forms**, and `as_exact_size` makes each `$skipping` or `$taking` of its
  source made knowing its back and length, a take of such items given their
  drop, `$taking(src, n, drop)`.
- A held one, `let held = v.into_iter().skip(1); held.rev()`, or a chain's
  side, which MIR holds in a variable before the chain is made, had no form
  left to read. **`back_stepped` records a `skip(n)` or a `take(n)` given held
  to what steps it from its back** (a `rev()`, generic code, or a chain's
  side through `with_stepped_sources`), never every one, so one made in
  place keeps reading as `.drop(n)` where nothing asks its back.
  `back_kept` makes a recorded one knowing its back and length where it's
  made, `$skipping` or `$taking` of its source made knowing them and closed
  to drop what it holds; `knows_its_back` and `knows_its_length` say so of its
  type (`counted_made_knowing`, the one read).
- **What's held beneath adapters made in place** (#790): `c.map(f).rev()` of
  a `let c = a.chain(b)` of the crate's iterators was refused unless something
  else stepped a map of that chain's type from its back. The `rev()` was given
  a map made in place, so nothing was held where it was given, and the
  chain arm records a chain only of items that drop. `back_stepped` now
  follows what a call asking its back is given through each call whose result
  is std's adapter of its first argument, to the std adapter a variable, a
  field or a place holds there (`held_beneath`), and records it as one given
  held is, a chain as the chain arm does. A `Vec`'s held `skip(1)`, mapped,
  and a held map, filtered, then reversed, are the same.

Two wrong answers in the runtime, unreachable before since no take of items
with a destructor was made knowing its back: `$taking`'s `rfold` trimmed its
source by `nth_back(len - n - 1)` and let the item found go undropped, where
std's `Take::rfold` drops it, so `ds().into_iter().take(3).rev().sum()`
never dropped `D(4)`; and both `$taking`'s and `$skipping`'s `nth_back` past
their end let what their source's `nth_back(len - 1)` found go undropped, as
std's `Take::nth_back` and `Skip::nth_back` drop it.

So an `enumerate()` of a `take()` of such items folded from its back, refused
since #651, runs: `enumerate_of_take_from_back_refused.rs` is
`enumerate_of_take_from_back.rs`, against native Rust, and the guide's
limitations list a `step_by()`'s alone. A `skip(n)` or a `take(n)` stepped
by its own `rfind`, `v.iter().take(2).rfind(p)`, is still refused, as the
guide lists: recording every one whose own back step is called, as a map is,
records std's compiled bodies' calls too, a `Rev<&mut Skip<..>>`'s
`next_back`, so a struct's held `skip` lent to a `for`, which never steps it
from its back, was given its back by `$knowing` (`held_adapters_by_ref.rs`), a
change of form for nothing.

`test/corpus/skip_take_of_droppers_from_back.rs` holds a skip's and a take's
`rev()` made in place and held, as a chain's side, `next_back`, `rfold`,
`nth_back` and `rfind`, `rev().nth` past either's end, a held `rev()` stepped
from both ends, a held one's `rev().len()`, and of numbers, against native
Rust in both builds; `test/corpus/held_beneath_adapters_from_its_back.rs`
holds #790's chain, mapped and filtered, a held `skip`, mapped, and a held
map, filtered. Mutations: `dropping-skip-uncounted` (retargeted),
`dropping-take-dropless` and `counted-made-knowing-unread`
(`lower/iterators/knowing.ts`), `counted-back-stepped-made-unknowing`
(`lower/mir/iter/backs.ts`), `counted-given-held-unrecorded`,
`counted-chain-side-unstepped`, `held-beneath-unrecorded`,
`held-beneath-chain-as-adapter` and `held-beneath-through-one`
(`lower/analysis/back_stepped.ts`), `taking-passed-undropped`,
`taking-nth-back-past-undropped` (`runtime/taking.ts`) and
`skipping-nth-back-past-undropped` (`runtime/skipping.ts`).

What's made knowing its back is decided of each type, so the corpus case
holds each way with items of a type of its own: `P` made in place, `C` a
chain's sides, `D` held. A `skip(n)` or a `take(n)` is recorded as a chain's
side alone among what `with_stepped_sources` takes in: a copy's, made in
place, is made knowing its back by the copy (`back_kept`'s copy arm, #646),
which records nothing. `held_chain_mapped_from_its_back.rs`'s second chain
is a chain of three now, a type of its own. #790's `held_beneath` recorded
the second chain's type, so `chain-under-back-stepped-map-unrecorded`
(`with_stepped_sources`' chain under a recorded map, #739) survived; it's
held by `chain_under_a_held_map_from_its_back.rs` now, a map whose closure
owns what has a destructor, of a chain MIR holds before the map is made,
reversed, which the mutant refuses, with a map's `rfold` given a call's
value, a held map's `next_back` and `nth_back` and a filter's map reversed,
against native Rust in both builds.
`back_kept`'s enumeration arm that makes its source knowing by
`as_exact_size`, which asks `knowing_held` of std's compiled struct first,
is held by `enumerate_of_droppers_from_back.rs` too (its mutation
`enumerate-of-take-from-back-unknowing`).

## Amended (2026-11-06, #718): a temporary a back step takes, made knowing its back where it's made

`next_back`, `nth_back`, `rfind`, `rfold`'s `try_rfold` and
`advance_back_by` take their receiver through a `&mut`, so MIR holds a
`skip(n)` or a `take(n)` made in place in a temporary to lend it:
`v.iter().take(2).rfind(p)` is `_11 = take(..); rfind(&mut _11, p)`. Its
form, JS's `.take(n)`, steps only forward, and the variable's read keeps
none of it, so the back step was refused, "a `Take<Iter<i32>>` as a
`DoubleEndedIterator`", while `rfold`, which takes it by value, made it
`$taking` where it is. Recording the type stepped from its back
(`back_stepped`) would change it wherever it's made, held ones too (iterI
found `held_adapters_by_ref.js` change for nothing).

**The local is the fact, not the type.** `IterLocals` records each local
a whole `&mut` of which is given as the receiver of what asks its back
(`back_stepped_temporaries`, `IterMethod::asks_its_back`: `steps_from_its_back`'s
and `next_back`), and that step's span. Where such a local is written,
a `skip(n)`, a `take(n)` or a `step_by(n)` the recognizers made in place is
made knowing its back as that step would make it, `as_double_ended` at its
span, `$taking($iter(v), 2)`, and marked so (`made_knowing_its_back`); the
mark is carried by the local, and `next_back` of one so marked is
`$nextBack`. Made where Rust makes it, nothing it reads is read later.
`$taking` and `$skipping` step from either end, so a local stepped forward
too is as right.

The recognizers' `take(n)` and `skip(n)` are marked made in place
(`Known::InPlace`) where they're made, as std_calls' adapters are: so
`knowing_held` never gives a JS `.take(n)` std's compiled `Take`'s steps,
which threw `it.back is not a function`. One held, std's struct, `$held`
of the crate's iterator, or closed to drop what it holds, is as it is; one
of a `Vec`'s items with a destructor is made so too, by afdd70519's
counted arm. `rposition`, which asks the length too, is
left refused: no mark says a value knows its length.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted. Held by corpus
`skip_and_take_stepped_from_their_back_in_place`, against native Rust in
both builds; no other snapshot changes, and both iterator slices pass.
Mutations `back-stepped-temporary-unrecorded` (`mir/temporaries.ts`),
`back-stepped-temporary-made-unknowing` (`mir/assign.ts`),
`back-stepped-temporary-unmarked`, `back-stepped-temporary-of-any-adapter-made`,
`back-stepped-temporary-held-made` (`mir/iter/backs.ts`, by its snapshots),
`next-back-of-marked-unread`, `take-made-unmarked-in-place` (`mir/iter.ts`),
`skip-made-unmarked-in-place` (`mir/iter/closing.ts`),
`next-back-asks-no-back` (`recognition/iterators.ts`).

## Amended (2026-11-06, #718): a `step_by(n)` of a `Vec`'s items with a destructor, and one counted

A `step_by(n)` of a `Vec`'s items with a destructor is made
`src.filter((item, i) => { if (i % n === 0) { return true; } drop(item);
return false; })`, each it passes over dropped. `every_nth`, the reader
of `step_by`'s filter, read only the filter of items with none, so its
`rev()`, `rfold`, `nth_back`, `next_back` and `rfind` were refused, "a
`StepBy<IntoIter<L>>` as a `DoubleEndedIterator`" (guide limitations.md).
It reads both now. And a `step_by(n)` couldn't be counted at all:
`as_exact_size` had no arm for it, and `$stepByEnds` no `len`, so an
`enumerate()` of one folded from its back was refused, of numbers too.

**One maker, `step_by_knowing` (stepping.rs), for both questions:**
`as_double_ended`
and `as_exact_size` ask it, of either form, the filter or `$stepBy(src,
n)`: `$stepByEnds` of its source made knowing its back and its length,
whose `len` is std's exact hint. Of a `Vec`'s items with a destructor, its
source, `$closing($iter(v), $dropEach(drop))`, drops what `nth_back` passes
over as std's `vec::IntoIter::nth_back` does, front to back; 256 generated
programs of 1 to 10 items, steps 1 to 4 and mixes of `next`, `next_back`,
`nth_back`, `rfind` and `rfold` print as native Rust does. One its owner
holds, `$held(it)` (#206), which a temporary a back step takes holds, is
made so inside and held again (`held_of`, value.rs).

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted. Held by corpus
`step_by_stepped_from_its_back`, against native Rust in both builds; no
other snapshot changes, and both iterator slices pass. Mutations
`every-nth-of-droppers-unread` (`std_types/range.ts`), `step-by-made-uncounted`
(`iterators/knowing.ts`), `back-stepped-temporary-held-unread`
(`mir/iter/backs.ts`), `step-by-ends-uncounted` (`runtime/back_steps.ts`);
`step-by-made-unstepped` and `step-by-made-by-nth-backless` name their
lines as they now read, in `iterators/stepping.ts` with their code.
Still refused: `len()` of a `step_by(n)` of a slice's items made in
place, which `len_in_place` counts only of a range.

## Amended (2026-11-06, #718): a `rev()` of a type parameter given as it is

A type parameter a struct holds is given as it is (ADR 0411), its value
the caller's, stepped by its dictionaries. Its `rev()` was
`$iterator(it, IDoubleEndedIterator.next_back)`, which knows no back, so
one the crate steps from its back, `self.0.rev().nth_back(1)` of `impl<I:
DoubleEndedIterator> Held<I>`, was refused. **It's made knowing its back
where it's made (`rev_called`): `$rev` of the JS iterator its dictionaries
step, `given_js_iterator`'s `$given(it, IIterator, boxed,
IDoubleEndedIterator)`**, which `$rev` steps from its back by its forward
steps, `nth_back` its `nth`, the crate's own where it writes one, as std's
`Rev` passes them on, closed to drop it as `Rev` drops it. What its
closing drops needed the fix of the same day to ADR 0421: of a `Vec`'s
items with a destructor, what its `nth` passes over is dropped as one
slice. The issue's note that `$given` gives no `of` is `$givenDrop`'s
(#766).

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted. Held by corpus
`rev_of_a_type_parameter_held_as_it_is`, of numbers, a map, a `Vec`'s
items with a destructor and the crate's iterator whose `nth` is its own,
against native Rust in both builds; mutation
`rev-of-given-param-unknowing` (`mir/iter/backs.ts`).

## Amended (2026-11-06, #823): #383's enumeration rule is back, and a `step_by(n)` from its back drops what it skips

766055774 (#764) removed #383's rule, an enumeration of items with a
destructor recorded stepped from its back, as no corpus case or slice
needed it. The matrix's full rows did: an `enumerate()` folded from its
back of the crate's own iterator under a `rev()`, a `skip(n)`, a `take(n)`
or a `step_by(n)`, and of a reversed `Vec`'s `step_by(n)`, 19 cases, threw
`it.back is not a function` in the default build, the `rev()` made in place
`$iterator(it, nextBack)` with no back. **The rule is back:** such an
enumeration, held in a variable to drop its items, is made knowing its back
where it's made, its source too. `take(n)`'s and `step_by(n)`'s forms, which
were refused before 766055774, run now. `enumerate_made_in_place_from_back`'s
`rev().map(f)` reads `$mapEnds($rev($enumerate(..)), f)` again, the rule's
form, the reading #764 traded the rule for.

The `step_by(n)` forms showed what `$stepByEnds` lacked: what a step
from the back skips was dropped by its source's own `nth_back` alone,
which a `Vec`'s does as one slice, but the crate's iterator's default, one
`next_back` at a time, didn't, so its skipped items were never dropped.
**`$stepByEnds(it, n, drop)` is given its items' drop** (value.rs'
`stepped_by_ends`, of `$stepBy(src, n, drop)`'s form too), and `$stepBy`'s
back step asks the source's own `nth_back` where it drops what it passes,
`ownNthBack`, else steps each from the back, dropping it, as std's default
`nth_back` does.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. Held by corpus
`enumerate_of_own_iterator_folded_from_its_back`, of the matrix's 19 rows'
shapes and steps past two or more of a `Vec` and of the crate's iterator,
against native Rust in both builds, and the 19 rows themselves; 326
generated `step_by` programs of a `Vec`'s and the crate's iterator's items
with a destructor print as native Rust does. Mutations
`enumerate-of-droppers-unrecorded` (`analysis/back_stepped.ts`),
`step-by-back-slice-dropped-each`, `step-by-ends-drop-unpassed`
(`runtime/back_steps.ts`), `step-by-ends-undropped`
(`iterators/stepping.ts`); four others name their lines as they now read.

## Amended (2026-11-06, #814): a counted adapter is stepped from its back by what holds it, not its type

Two rules decided whether a `skip(n)`, a `take(n)` or a `step_by(n)` is
made knowing its back. The per-type set recorded one given held to what
asks its back, and a chain's side (afdd70519); `IterLocals` recorded a
temporary lent to a back step (#718's fourth part). Two authorities for
one question, which AGENTS.md refuses. Neither alone could hold both:
recording the type of every one a back step asks changed what else of
that type was made, a field's `skip` given `$knowing` at a plain `for`,
and four matrix rows' drop order. One type has many values, of which only
some are stepped from their back.

**One fact, by the MIR identity of what holds the value:
`back_stepped::stepped_locals`, each `(body, local)` the crate steps from
its back, and how it holds it.**
- `Lent`: a temporary whose `&mut` is given as the receiver of what asks
  its back, a `DoubleEndedIterator` clause of the callee's of its type,
  `nth_back(&mut _11)`'s. Its value, made in place, is made knowing its
  back where it's assigned, by its form (`made_knowing_where_held`).
- `Given`: a variable's value given whole to what asks its back,
  `rev()`'s or generic code's, through each move MIR makes of it; a
  chain's side, of a chain stepped from its back; a variable held beneath
  an adapter made in place that a step asks, `s.map(f).rev()` of a held
  `s` (#790). Not a function's return place: one a function gives behind
  an `impl` is std's struct (ADR 0919), which steps from its back by its
  compiled impl. Its maker, `back_kept`, knowing it by the local its call makes its
  value into (`made_into`, set where the call's terminator is lowered),
  makes it `$skipping` or `$taking` and marks it made knowing its back.

Not a temporary given whole, made in place where it's stepped, whose form
the step reads, `v.slice(2).toReversed()`; nor a variable lent, std's
struct, whose compiled back the step asks. The per-type set records no
counted adapter, and the type questions `knows_its_back` and
`knows_its_length` don't answer of one: its value's mark does, carried by
its locals. A `rev()` of one that knows its length, made knowing its back
or std's struct given its compiled `len` (`knows_its_given_length`), is
marked so where it's made (`rev_called`), and `len_in_place` reads the mark
of a variable that holds it, held to drop its items (`held.rev().len()`). `IterLocals` holds no back-step fact of its own. The
architecture test "a counted adapter is stepped from its back by
back_stepped's stepped_locals alone" holds it.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. The
whole corpus, the snapshots, both iterator slices and #823's 19 matrix rows
run as before, and 138 generated programs (each counted adapter, of numbers
and of items with a destructor, made in place, held, given behind an `impl`
or a chain's side, stepped back six ways) run, are refused or are wrong as
on main, in both builds; one snapshot reads better, `skip_take_of_droppers_from_back`'s
`$rev(held).map(f)` summed forward, where the type said `held` knew its back
and its map was made `$mapEnds`. Mutations `counted-given-held-unrecorded`,
`counted-chain-side-unstepped`, `back-stepped-temporary-unrecorded`,
`back-stepped-temporary-made-unknowing`, `counted-back-stepped-made-unknowing`
and `counted-made-knowing-unread` name the new fact's lines;
`stepped-local-held-beneath-unasked`, `stepped-local-origin-unread`,
`counted-made-knowing-unmarked`, `rev-of-counted-knowing-unmarked` and
`rev-of-given-counted-unmarked` are new, the last two held by corpus
`counted_adapter_given_back_from_its_back`; `next-back-asks-no-back` goes with `asks_its_back`.

## Amendment: std's compiled struct has a back only where it's given one (2026-11-06, #856)

ADR 0262's case: results change, so Rust's behavior is kept. Where the
crate folds a zip of a type from its back elsewhere, `knows_its_back`
says each value of that type knows its back, as its maker makes the one
made in place `$zipEnds`. But a compiled `Zip`, std's struct, `iterator_zip(..)`,
is made as it is, with no back until it's given its compiled one
(`$knowing`). A map or a filter of it was `$mapEnds` or `$filterEnds` of
the struct as it was made, so its `rfold` threw where Rust folds it.
`has_a_back` (backs.rs), what the adapters' arms ask before making their
source knowing its back, now asks the value too: an array's, or one of a
type that knows its back, but std's compiled struct, which has one only
given it, `$knowing`'s or made knowing it. Found by #818's `pairs` beside
`vec | zip | rfold`; bisected on a clean `main`. Corpus
`zip_of_its_type_stepped_from_its_back_elsewhere`; mutation
`compiled-struct-taken-as-knowing-its-back`. The per-type fact itself is
#851's to retire.

## Amendment: an adapter held across what its step's argument runs is made knowing its back where it's made (2026-11-06, #811)

ADR 0262's case: none, as the JS ran as Rust does or was refused; this
removes a refusal. `u.into_iter().map(|x| x * 10).nth_back({ k += 1; k - 2
})` was refused in the default build, though `v.iter().map(f).next_back()`
runs: MIR holds the map in a variable across the statement the argument
runs, its `&mut` lent to `nth_back` (or the map given whole, `rfold({ k
+= 1; k }, g)`), and the lowering writes it out there, a `const` of its
form, which the step then can't read; a map's type knows no back, so it
was refused. `stepped_locals` now records such a temporary `Lent`, as it
records a counted adapter's lent temporary (#718, #814):

- **Held across what acts.** `held_across` follows MIR from where the
  temporary is lent (its `&mut`) or made (its call's return) to its step's
  call, along `Goto`s and overflows' `Assert`s, and says whether a
  statement between writes a variable or a place, or whether a call, a
  switch or a drop runs between. Moves, borrows and arithmetic into
  temporaries don't act: a lent map whose `next_back()` follows it is read
  in place, `$nextBack($iter(v))` mapped, as before.
- **Given whole too.** A temporary given whole, held so, is recorded
  `Lent` once the fixpoint ends, apart from `asked`, so what it holds
  beneath is asked as before.
- **Where it's written.** `written_knowing_its_back` (backs.rs) makes a
  `Lent` temporary knowing its back where it's written, assigned or
  written at last by `flush` before what runs next, `$mapEnds` or
  `$filterEnds`, its mark carried by its local. `made_knowing_where_held`
  is any adapter's, the counted question being the analysis's.

Proof: 57 generated programs (a map, a filter, an enumeration, a `rev()`,
a `skip`, a `step_by`, a `zip`, a map of a map, a `filter_map` and a copy,
of numbers and of items with a destructor, stepped by `nth_back`,
`rfold` and `rfind` past an argument's statement) run as native Rust in
both builds, where `main` refused two; the corpus, the snapshots, both
slices and the rows #818's gate names as before. Corpus
`adapter_stepped_from_its_back_past_an_arguments_statement`; the guide's
limitation is gone. Mutations `held-across-nothing-acts`,
`held-across-its-own-step`, `held-lent-unrecorded`,
`held-whole-unrecorded` and `flushed-made-as-it-is` are new; four
retargeted. `back-stepped-temporary-held-made`, which let a value not made
in place be made again, is retired as equivalent: a counted adapter's
`Lent` temporary held as std's struct reaches that guard, in both builds,
but made again it's what it was, so no test can tell; the guard stays, as
what it keeps from `as_double_ended` could only be refused there.

## Amendment: what asks whether a value has a back never trusts its type over its form (2026-11-06, #851 phase A)

ADR 0262's case: results change where it applies, so Rust's behavior is
kept. #850 and #856 were one shape: a maker left a value as it was made,
a `cloned()`'s JS `.map(clone)`, a compiled `Zip` with no `$knowing`, while
what asked whether it has a back trusted its type, which back_stepped's
set said steps from its back because another value of it did; it then
stepped a back the value hadn't and threw. A third, an `inspect(f)` of a
chain stepped from its back elsewhere, made JS's `.map(f)` beneath a
`$mapEnds`, threw the same way on `main` (found by #851's probe of 13
adapters over such a chain, each stepped back four ways).

`has_a_back(ty, value)`, what each adapter's arm and each back step asks
before making its source knowing its back, now withdraws trust where the
value's form says it has none: JS's own iterator helpers, `.map(f)`,
`.filter(p)`, `.take(n)`, `.drop(n)`, `.flatMap(f)`, which step only
forward, or an adapter of one whose back is its source's (`$mapEnds`,
`$filterEnds`, `$enumerate`, `$lent`, `$closing`, `$held`, `$taking`,
`$skipping`): value.rs's `form_lacks_a_back`, the owner of an iterator
value's form, beside `knows_its_back_by_form`, what a form says it has.
What its form doesn't say, a variable's, a field's, a parameter's, a
function's result, its type says, as what made it asked the same set.
Such a value is made knowing its back where it's asked
(`as_double_ended`) or refused, never stepped by what it lacks. This is
#851's first phase: the makers keep the set, and each moves to a value's
own fact next, its misses a conversion or a refusal now, never a wrong
answer.

Measured first: across the whole corpus no value is trusted today whose
form lacks a back, so its JS is as it was; the matrix and the conformance
table in full, both builds, as before. Corpus
`inspect_of_a_chain_stepped_from_its_back`, which throws on `main`.
Mutations `has-a-back-trusts-the-type` and
`form-lacks-no-back-of-js-helpers`; #850's `copied-items-mapped-backless`
is still caught, its maker's own fix kept.

## Amendment: a map is made knowing its back by its value's fact, not its type's (2026-11-06, #851 phase B, maps)

ADR 0262's case: none where its JS changes, as it ran as Rust does; what
was refused stays so or runs. The first kind moved off back_stepped's
per-type set is the map: whether a `map(f)` is made knowing its back is
now asked of its value, `stepped_from_its_back` (iterators/back_facts.rs,
the facts' one reader): the local the call being lowered makes its value
into (`made_into`) or is given (`made_from`, as a chain made in place is
built at its consumer), through the `&mut` that lends it (`borrows`),
recorded by `stepped_locals`; or a type whose question is a type's
(`typed`): one a function gives behind an `impl`, and a generic
function's of its callers' (#388). Other kinds keep the set, each moved
in turn; a length is the set's still (`by_set`, `knows_its_length`).

`stepped_locals` records what the set covered of a map's values:

- a variable lent to its step, `m.next_back()` of a `let m = ..map(f)`,
  a JS form its maker makes knowing its back (a counted adapter so held
  is std's struct, #718);
- a temporary lent or given whole where MIR holds it across what acts
  (#811), or the receiver of `DoubleEndedIterator`'s own method of a kind
  no form of whose own a step reads from its back where it's made
  (`back_stepped_adapter`'s), a map of the crate's iterator
  `count().map(f).rfold(..)`; or of an array's items or a closure with a
  destructor, or an enumeration of std's struct. Not a map of an array's
  items an `rposition` asks, whose step reads its form;
- a field a back step is given, lent or whole, `self.it.next_back()`:
  each value MIR moves into it as it makes the struct;
- a zip's sides as a chain's (#718, #792), and the `b` of a zip read by
  index, a map whose closure has a destructor (#775); an
  `ExactSizeIterator` clause of such a map, held to drop it (#385).

The marked value carries its fact past its maker: where a std adapter
method makes a map into a local that's given to what steps it from its
back, the value is marked made knowing it. What asks a value whether it
has a back reads its form down to what holds it (`knows_its_back_of`):
a wrapper, `$closing`, `$lent`, `$held`, `$holding` (iter.rs's
`held_by_holding`), an adapter whose back is its source's, down to a
variable whose type says it, never a bare variable of an array's
iterator, which may hold the array, nor std's compiled struct, which
has a back only given it (#856). `as_double_ended` returns such a value
as it is, and makes knowing again what holds one that isn't,
`$mapEnds`'s, `$filterEnds`' and `$holding`'s source.

The cross-check: where the value's answer and the set's differ,
`RUST_JS_BACK_STEPPED_CROSS_CHECK` says so on stderr while the set
remains (phase C removes both). Across the corpus 58 answers differ, in
17 cases; two change their JS, each a value of a stepped type that isn't
stepped itself or the same compiled struct given its back sooner:
`generic_map_folded_and_rfolded`, a map only folded forward, JS's own
`.map(f)` where it was `$mapEnds`; `backs_given_behind_impls`,
`$lent($knowing(it, ..))` where it was `$knowing($lent(it), ..)`. The
others' JS is as it was. The whole corpus, both builds, and the matrix and
the conformance table in full run as native Rust.

Mutations: `map-stepped-by-its-type`, `made-from-unread`,
`zip-sides-of-a-zip-unstepped`, `zip-by-index-b-unrecorded`,
`held-closure-map-unrecorded`, `own-method-receiver-unrecorded`,
`knowing-form-made-again`, `map-source-not-made-again`,
`bare-array-iterator-trusted`, `compiled-variable-trusted` and
`length-asked-per-value`, each held by the corpus case that showed the
rule was needed. Four older ones, `rev-made-knowing-read-by-form`,
`flat-map-closure-result-by-span`, `zip-by-index-any-map-asked` and phase
A's `has-a-back-trusts-the-type`, are retired as equivalent under the
value's fact: with all four and two rules of this phase's applied at
once, the corpus, the matrix and the conformance table in full
(`ITERATOR_MATRIX=all`, both builds) run as before. The two rules, an
`ExactSizeIterator` clause of a map whose closure drops, and remaking a
`$holding`'s source, are removed; the four older rules' code stays, as
the kinds still on the set may need them, until phase C retires the set
and them with it.

## Amendment: what #851's per-value facts left dead, removed (2026-11-08, #863)

#851's phases A and B moved what a value's back is from its type to the
value. Fourteen mutations of the rules before them then survived (#863).
Seven were the rules' own and only needed their tests named again:
`held-adapter-not-back-stepped` built again, its `made` kept of its type;
three name the corpus cases they fail now; three change a case's form, not
whether it's refused, and are its snapshot's (`snapshots: true`).

The other seven changed nothing a case shows: not the corpus, of either
build, nor the rows of the matrix and the conformance table their hunks
name. All seven at once failed one conformance case, `range | map | rfold
holding`: two of them were one rule written twice, a temporary written at
last before an argument's statement made knowing its back
(`temporaries.rs`'s `written_knowing_its_back`) and the range a back-stepped
map is made of made knowing it (`backs.rs`'s map arm, `as_double_ended`),
each covering the other. With the first and five more removed, every case
of the matrix, the conformance table, the points table and the corpus with
std's iterator structs runs as before, and no corpus snapshot changed. So
they're removed, with their mutations: the flushed temporary's
`written_knowing_its_back`, which the map arm owns; a flat map's closure
marked as giving what's made in place (`gives_in_place`,
`gives_made_in_place`, `Known::IterResultInPlace`, a body's
`result_in_place`); `with_stepped_sources`'s walk past a source's own
sources; `form_lacks_a_back`'s arm of JS's own helpers, `.map(f)`, which no
back step reaches; and tidy's `says` of an `&&`, which the decision tree
covers. `rev-of-zip-not-made-knowing` added an exception rather than
removing one, and is retired as equivalent. `back-stepped-source-raw`, of
the map arm that now owns the case alone, is caught by its corpus case,
`range_map_rfold_holding.rs`; `flatten-of-iterators-each-called` names its
line as it now reads.

## Amendment: a generic function's `impl`, in its own terms (#993)

What a caller steps from its back that a function gives behind an `impl`
is made knowing its back where it's made, but the set had it in the
caller's terms, `Filter<Range<u32>, {closure}<bool>>` of a call of
`evens_or_tens::<bool>`, where the generic maker's body asks of
`Filter<Range<u32>, {closure}<B>>`: its filter made as JS's own, which
`Two`'s `next_back` found no back of, `it.back is not a function`. Each
type of the `impl`'s hidden type is recorded in the maker's terms too, as
`type_of` of the `impl` gives it, matched to the caller's by instantiating
it with the `impl`'s arguments (`made_generically`), and given to the
maker's own set, which its callers instantiate in turn (#388). Corpus case
`generic_impl_stepped_from_its_back`.

## Amendment: a filter's too (2026-11-08, #851 phase B, filters)

ADR 0262's case: none where its JS changes for what ran as Rust does; two
snapshots change, a filter only stepped forward, of a type stepped from its
back elsewhere, now JS's own `filter` or `$filterDropping` where it was
`$filterEnds`. Whether a `filter(p)` is made knowing its back is its
value's fact too (`back_by_value`, back_facts.rs, the one answer of what's
asked and what's marked). What `stepped_locals` records grows by what the
set covered of a filter's values:

- the source of a `rev()` held in a variable, std's compiled `Rev`, which
  steps it by its `next_back` (the set's `held_rev`, retired);
- a value given beside a `&mut` of a local stepped from its back, of its
  type, which the callee may write there, `replace_in(&mut it, next)`
  (#537), through a reborrow `&mut (*_r)` too;
- what a call gives that's stepped from its back, lent or given whole,
  made by the crate's function or a closure called through its `Fn` trait
  (`made_by`): its return place, and the call's value marked so where it's
  made;
- the source a std struct holds, which asks the dictionary of what it
  holds by its type alone, `mapDoubleEndedIterator_nextBack`'s: recorded as
  a type's question (`typed`);
- a field a back step reads: a read of it marked made knowing its back, as
  a local's value is (`BackStepped::fields`, places.rs).

A temporary lent to its step, `v.iter().filter(p).next_back()`, is marked
too, not only one given whole. The set's rules a map's and a filter's
values no longer reach, `held_rev` and two mutations of what an adapter of
an array's items with a destructor asks (`held-adapter-asked-itself`,
`rev-of-dropping-map-unasked`), are retired with the set in phase C.
Corpus case `filter_given_back_stepped_from_its_back`; the full iterator
matrix, conformance, points and unwind tables, the metamorphic net and the
corpus in both builds run as before.

## Amendment: a chain's and a copy's too (2026-11-08, #851 phase B, chains and copies)

ADR 0262's case: none where its JS changes for what ran as Rust does; two
snapshots change, a chain only stepped forward, of a type stepped from its
back elsewhere, now `$lazyChain(..)` mapped by JS's own `map` where it was
`$mapEnds`, and a forward-only chain of the crate's iterators no longer
given an unused `nextBack`. Whether a `chain`, a `copied()` or a `cloned()`
is made knowing its back is its value's fact (`back_by_value`), as a map's
and a filter's are. What `stepped_locals` records grows by what the set
covered of their values:

- what an asked adapter made in place is made of, asked in turn, down to
  what a variable holds, a `let c = a.chain(b)` beneath
  `c.filter(p).map(f).rev()`, marked where it's made (the set's
  `held_beneath`, #790);
- what an asked `copied()` or `cloned()` is made of, which it is in JS,
  and the source of an adapter whose closure has a destructor, which MIR
  holds before the adapter is made;
- a temporary chain of items with a destructor lent to its back step,
  which MIR holds to drop them (#718), and a `cloned()` lent to
  `DoubleEndedIterator`'s own method, `a.chain(b).cloned().rfind(p)`, as a
  `copied()` is.

A `copied()` or a `cloned()` made knowing its back has its source made
knowing it too, as a map's is: a `step_by(2)`'s, `$stepByEnds`, where
`$mapEnds` of its forward-only `values().filter(..)` threw (the iterator
matrix's `step_by.cloned | rfold`, `fuse.cloned` and `peekable.cloned`,
caught before this landed). A variable's `cloned()` of items that need a
clone, held where a closure captures what drops, stays refused, as on
`main`: a first draft took it for its source, the array's iterator,
`$foldBack($iter(tmp))`, which stepped nothing (the matrix's `vec iter |
cloned | rfold | captures`); a diagnostics case holds the refusal. A value
made by any method of std's `Iterator` trait, a `chain` too, which the list
of iterator methods lacks, is marked made knowing its back where its local
is stepped.

The `len()` of a value made before asks whether it's an array's `$iter`
(`steps_an_array`), whose `len()` is `it.end - it.at`, apart from whether
it has a back (`has_a_back`), which a `next_back` asks: one answer to each
question, where one served both.

The readability baseline moves by the new corpus case's `fuse()` and
`peekable()` given their compiled backs, `$knowing(..)` (`Iterator.from`,
8.1 to 8.2 in 10,000 lines), as they were made before.

## Amendment: an iterator a generic struct's field holds (#1013, 2026-11-08)

ADR 0262's case: results change, so Rust's are kept. The metamorphic net's
round 2 (#1008) holds each iterator in a field of a generic tuple struct,
`Held(v.iter()).0.rev()`, where three things threw:

- `rev_items` took a variable's or a named field's array iterator for the
  iterator, but a tuple struct's field reads `held[0]`, which it took for
  its array, `held[0].toReversed()`: a read of an index is one too,
  `$rest(held[0])`;
- `BackStepped::fields` is a field of the struct, whatever it holds, so
  `Held(range).0` stepped from its back marked the read of `Held(map).0`,
  std's compiled `Map` (ADR 0871), made knowing its back, and it was given
  no compiled back, `$rev(mapped[0])`: the read is marked only of what its
  maker marks, never std's compiled struct, as `structure.rs` marks it;
- a value given beside a `&mut` of a field stepped from its back,
  `replaced(&mut held.0, next)`, which the callee may write there, is made
  knowing its back as one beside a local's is (#537), through a reborrow
  of the field's `&mut` too.

The same field fact fixed three of the library rewrites (#1024):
`filters_held_double_ended`, `generic_impl_stepped_from_its_back` and
`own_back_methods_given_dictionaries`. Corpus case
`iterators_held_in_a_generic_structs_field`; mutations
`tuple-field-iterator-taken-for-its-array`, `stepped-field-compiled-marked`,
`beside-stepped-field-unmarked` and `field-reborrow-lends-nothing`.

## Amendment: what crosses a library's boundary (#1024, 2026-11-08)

ADR 0262's case: results change, so Rust's are kept. Two crates, each
compiled on its own (ADR 0100), each blind to where the other steps a value
from its back (the metamorphic net's library rewrites, #1008):

- **What a library gives behind an `impl DoubleEndedIterator`.** Its
  consumers step it from its back where it can't see, so in a library each
  public function's return behind such an `impl` is asked as one the
  crate's own callers step is: each adapter in it is made knowing its back
  where it's made, a filter `$filterEnds`, where JS's own `.filter` threw
  `it.back is not a function` (`given_behind_an_impl`, shared with the
  in-crate rule). The signature's late-bound regions are erased with the
  rest, as the made value's type has them.
- **What a crate gives a library's generic function.** A parameter that's
  a std adapter of a source the function's clauses bound by
  `DoubleEndedIterator`, `back<I, P>(it: Filter<I, P>)`, is one its body
  may step from its back; the crate can't see the body, so the adapter it
  gives is made knowing its back where it's made, and the library, which
  can't see its callers, takes each such parameter for one so made
  (`library_backs`, read on both sides from the signature alone: one
  contract). A library with no caller of its own was refused, `next_back()`
  of a generic `Filter`.

Std's compiled structs a library gives (`iterator_map`, `iterator_chain`)
were right already: their consumers step them by their dictionaries.
Two-crate case `iterators_stepped_from_their_back_across_crates`; mutations
`library-impl-backs-unmade`, `library-given-backs-unmade` and
`library-own-params-unstepped`.

## Amendment: a zip's too (2026-11-09, #851 phase B, zips)

ADR 0262's case: none where its JS changes for what ran as Rust does; three
snapshots change, a zip only stepped forward, of a type stepped from its
back elsewhere, now `$trustedZip(..)` where it was `$trustedZipEnds(..)`,
and a map of a `rev()` stepped forward JS's own `.map` where it was
`$mapEnds`. Whether a zip is made knowing its back is its value's fact
(`back_by_value`), and so is whether it knows its length, which only
`$zipEnds` does: `knows_its_length` reads the zip's back by its value, not
by the set, so `zip(..).len()` of one made forward never calls a `len` it
hasn't. What `stepped_locals` records grows by what the set covered:

- a zip whose length `len()` asks, a side of which MIR holds in a variable
  before the zip is made, no form left to count it in place, is made
  knowing its back and length, `$zipEnds`; not one of an array's iterator
  a variable holds, counted as it is, `Math.min(it.end - it.at, ..)`;
- a shared borrow, `len(&self)`'s, lends what it borrows, as a `&mut`
  does, in `stepped_locals` and in the borrows a call's locals are read
  through.

## Amendment: an enumeration's too (2026-11-09, #851 phase B, enumerations)

ADR 0262's case: none where its JS changes for what ran as Rust does; one
snapshot changes, `enumerate_of_a_reversible_source`'s `count()` of an
enumeration of std's compiled `Peekable` and `Fuse`, stepped only forward,
now `$enumerate(it)` where it was `$enumerate($knowing(it, ..))`, as only
another value of its type is stepped from its back. Whether an
enumeration is made knowing its back is its value's fact
(`back_by_value`). What the set covered, an enumeration of std's struct a
`rev()` asks and `back_stepped_adapter`'s of one, is the value's own rule
(`unread`, which marks one held where it's stepped from its back), so the
set's two are gone, with their mutations; `unread`'s has one,
`enumerate-of-compiled-struct-held-unread`.

## Amendment: a flatten's too (2026-11-09, #851 phase B, flattens)

ADR 0262's case: none where its JS changes for what ran as Rust does; five
snapshots change, a `flatten()` or a `flat_map` only stepped forward, of a
type stepped from its back elsewhere, now `$flatMap(..)` where it was
`$flattenEnds(..)`, and what it's made of with it: a map of the crate's
iterator under it JS's own `.map`, its `$iterator` given no `nextBack`.
Whether a flatten is made knowing its back is its value's fact
(`back_by_value`). `$flattenEnds`'s own `count`, which a `skip`'s passes on,
was read only by a case now `$flatMap`; `flatten_count_takes_each_inner_count`
asks it of one made in place and given to generic code that steps it from
its back, then skips (`flatten-ends-count-not-passed-on`). The variable
rule `compiled-variable-trusted` guards is held by `returned_impl_ends`, as
`flatten_from_its_back`'s flatten no longer reaches it. Std's compiled
`FlatMap` held in a variable, stepped from its back, then skipped, counts by
`next`, not each inner's own `count`, as before this: #1048.

## Amendment: lengths too (2026-11-09, #851 phase B, lengths)

ADR 0262's case: none where its JS changes for what ran as Rust does;
four snapshots change, each for the better: a map only collected or
stepped forward, of a `rev()` of a map stepped from its back elsewhere,
or of std's compiled struct, is JS's own `.map` where it was `$mapEnds`,
and a held copy's `len()` is `held.len()` where it was `held.end -
held.at`. `knows_its_length` no longer asks back_stepped's set: its
questions of a back are its value's, and the `by_set` switch is gone.
What the set said of a type's length, a use now asks of the value:

- `knows_its_length_of(value, ty)`: its type's answer, or its maker's
  mark, `made_knowing_its_length` (`Known::IterKnowsLength`, kept by a
  variable, `$held` and `$holding` as the back's is), or, of a map, a copy,
  an enumeration or a `rev()` of an array's items, its back by its form or
  mark, whose every form over a `$iter` knows its length. `as_exact_size`
  and `len_in_place` (of a variable, not std's compiled struct, which has a
  `len` only given it) ask it.
- A map is made knowing its length, `$mapEnds` marked so, where its type
  says so as before, or where beneath its source is a variable its maker
  made knowing its back or length, `$rev(mapped)` of a `mapped` stepped
  from its back elsewhere (`maps_what_knows_its_length`); not one made
  knowing it in place, nor an array's own, which `len_in_place` counts.
- `stepped_locals` records a map whose closure has a destructor where a
  bound asks its length, as the set's rule of its type did (#385).

`as_double_ended`'s remaking of a `$mapEnds` source made for its length
alone is gone: nothing reaches it, in the corpus or the matrix. Seven of
the set's rules of lengths (`closing-map-asked-unrecorded`,
`closing-map-length-unasked`, `zip-by-index-holding-map-unasked`,
`held-given-of-items-native`, `held-through-borrows-unseen`,
`zip-sides-not-back-stepped`, `cloned-source-not-recorded`) no longer
change what's made, their mutations surviving, as the twelve phase B left
before: phase C retires them with the set.

## Amendment: a `filter_map`'s and an `inspect`'s too (2026-11-09, #851 phase C)

ADR 0262's case: none; the corpus's JS is the same. Whether a
`filter_map` or an `inspect` is made knowing its back is its value's fact
(`back_by_value`), as a map's and a filter's, whose forms they're made
of, are; a variable that holds one is marked so (`structure.rs`), and a
`rev()` of it reads the mark. Four more of the set's rules no longer
change what's made, their mutations surviving the whole corpus
(`held-given-unrecorded`, `path-call-uncalled`, `chain-sides-unstepped`,
`chain-side-a-unstepped`): phase C retires them with the set. Left on
the set: a `rev()`'s, a `skip(n)`'s, a `take(n)`'s and a `step_by(n)`'s.

## Amendment: a `rev()`'s too (2026-11-09, #851 phase C)

ADR 0262's case: none where its JS changes for what ran as Rust does;
three snapshots read better: a `rev()` only stepped forward, of a type
stepped from its back elsewhere, reads its source's back as its own,
`$iterator(range, $rangeNextBack)`, where it was `$rev($iterator(range,
$rangeNext, false, $rangeNextBack))`, and a reversed range's own `len`,
`last` and `nth_back` read its bounds as they are, `$rangeLen({ start: 1,
end: 6 })`, where they read `$rev(..).of`. So `$rev` no longer passes on
its source's `of`, which nothing reads, in the corpus or the matrix. One
more of the set's rules (`stepped-sources-any-adapter`) no longer changes
what's made. Left on the set: a `skip(n)`'s, a `take(n)`'s and a
`step_by(n)`'s.

## Amendment: a `skip(n)`'s, a `take(n)`'s and a `step_by(n)`'s too (2026-11-09, #851 phase C)

ADR 0262's case: none; the corpus's JS is the same. Whether a counted
adapter is made knowing its back is its value's fact (`back_by_value`),
as each other adapter's is: every kind back_stepped's set covered is its
value's now. Its maker makes none knowing its back: what steps it so makes
it so where it's made, `$taking`, `$skipping`, `$stepByEnds` (ADR 0542),
or it's std's struct where MIR holds it (#718). So a variable that holds
one isn't marked made knowing its back (`structure.rs`), which would have
taken `$stepBy` or std's struct for a form that has one.

## Amendment: the lowering reads no type's set (2026-11-09, #851 phase C)

ADR 0262's case: none; the corpus's JS is the same. Every kind
back_stepped's set covered is its value's now, so `stepped_from_its_back`
asks the value alone, `made_into`'s and `made_from`'s marks, and
`typed`'s types, whose question is a type's; the set's fallback and the
`RUST_JS_BACK_STEPPED_CROSS_CHECK` comparison are gone, and so is
`BackStepped::types`. `back_by_value` stays as what it is now: which
values' maker makes a variable that holds them marked (`structure.rs`,
`places.rs`), std's adapters, not the crate's own iterator. The set
`back_stepped` computes still seeds `stepped_locals` alone, a value made
of what's stepped, made stepped too (`made_stepped`): eight cases, a
chain of adapters, a `take` of a `rev()` of droppers among them, lean on
it. Phase C's last step gives them rules of their own, and retires the set.

## Amendment: no type's set (2026-11-09, #851 phase C)

ADR 0262's case: none where its JS changes for what ran as Rust does; one
snapshot reads better, a zip's side only stepped forward JS's own `.map`
where it was `$mapEnds`. back_stepped computes no set of the types the
crate steps from their back. What's stepped is each value's, by the MIR
identity of what holds it (`stepped_locals`), and its seeding by the set is
gone: what an adapter asked from its back holds, where MIR holds it in a
variable, is marked by the adapter's being asked, not by its type's being in
the set, a chain's sides and a zip's (`chain && made_asked`) and the source of
a map whose closure has a destructor (`spilled`). What `back_stepped` gives
is `typed` alone, the types whose question is a type's: one behind an
`impl`, a library's (#1024), and a generic function's own adapter of its type
parameters it steps from its back, each of its callers' (#388), with the
source of each that steps from its back where its source does.

The set's rules no value needed are gone: a zip read by index's `b`
(#775), a generic function's `impl` in its own terms (#993), what a variable
given holds and what's held beneath what's made in place (#484, #790), a
chain of droppers asked (#718), a map whose closure drops asked or asked its
length (#385), and `made_generically`; with them twenty mutations that
survived the whole corpus, each removal held by the corpus, the iterator
tables, the metamorphic net and the two-crate pairs. `back_by_value` is
`made_knowing_its_back_by_its_maker`: the adapters whose maker makes them
knowing their back, whose variables and fields its mark is kept by, not a
zip's, a `skip(n)`'s, a `take(n)`'s nor a `step_by(n)`'s, made so where
they're stepped (ADR 0542). An architecture test holds it: `BackStepped`
has `typed`, `borrows`, `locals` and `fields` alone, `back_stepped` gives
one set, and nothing compares a value's answer with a type's.

## Amendment: what no type's set leaves to guard (2026-11-09, #1059)

With no type's set, no value is made as it is because its type is
stepped from its back elsewhere, so two guards against that are gone:
`has_a_back`'s of std's compiled struct, which had a back only given it
(#856), and `enumerates_a_compiled_struct`'s exclusion of a held compiled
struct (#712); neither changes what the corpus, the iterator matrix or the
metamorphic net make. `given_stepped`'s fallback for a type parameter whose
bounds give no back stays: an enumeration of a type parameter's iterator
generic code makes, mapped and collected, reaches it, now a corpus case of
its own (`enumerate_of_a_made_type_parameter_collected.rs`). Three
knowing-its-back mutations name the cases that catch them.

## Amendment: an enumeration's rule of its own is gone (2026-11-10)

`back_stepped_adapter` kept a rule of its own for an enumeration, of items
with a destructor or of a map or a filter of an array's items that drops
them (#383, #764, #823): made knowing its back where it's made. Since #851
gave each value its own facts, the walk's `unread`, an enumeration of
std's struct or of what drops, marks each such value itself, and the rule
answered nothing: with it gone, every corpus case's JS is the same, in
both builds, #823's and #764's cases among them, against native Rust, and
the iterator matrix's and conformance table's rows the change names pass.
Its two mutations, `enumerate-of-a-dropping-map-unrecorded` and
`enumerate-of-droppers-unrecorded`, survived on `main`, so they're gone
with it. What's left of it is `unread`'s, which its own mutations hold.
## Amendment: a chain made where it's stepped from its back (2026-11-10, #1118)

A chain made where its own back step asks it, `a.chain(b).nth_back(1)`,
`next_back()` or `rfind(p)`, was refused, "a `Chain<I, I>` as a
`DoubleEndedIterator`, which rust-js can't step from its back", in generic
code and of ranges and a `Vec`'s items alike, where a held chain, its
`rfold`, its `rev()` and a map over it ran. MIR lends the temporary to the
step, which reads no form of a chain's: the walk marks such a temporary
where `back_stepped_adapter` says no form of its own steps from its back,
a `rev()`'s, a zip's, a flatten's, a map's, and a chain wasn't one. It is
now, where it's made here: `$lazyChain` of its sides made knowing theirs,
as one an adapter asks beneath it is. One a function gives behind an
`impl` stays std's `Chain`, which steps from its back by its own methods
(ADR 0919). No corpus case's JS changed. Held by
`test/corpus/chain_made_here_stepped_from_its_back.rs`, against native Rust
in both builds: generic code's, ranges', a `Vec`'s items, what makes an
iterator, the crate's own double-ended iterator, and items with a
destructor, each passed over dropped as std's `nth_back` drops it.
Mutation `chain-made-here-backless` (`lower/analysis/back_stepped.ts`).

## Amendment: a cloned chain lent to its back is a chain made here (#1109)

fab382391 (#1002) marked a chain of a type the crate clones apart, lent to
its own `next_back()`, as made knowing its back (`cloned_chain` in
`back_stepped`'s `unread`), for `cloned_chain_stepped_from_its_back.rs`.
e9415f2d1 (#1118, above) since marks every chain made here that its own
back step asks so (`back_stepped_adapter`), cloned apart or not. So the
first decided nothing that changed any output: its mutation
`cloned-chain-lent-unmarked` survived, and its case passes in both builds
without it. With #1118's rule taken out too (`chain-made-here-backless`),
`cloned_chain_stepped_from_its_back.rs` fails beside
`chain_made_here_stepped_from_its_back.rs`, where #1118's alone fails only
the latter: #1118's is the one answer, and the `unread` test is removed
with its mutation. `cloned_chain` stays, as a chain cloned apart that's
asked beneath an adapter is still its own.

## Amended (2026-11-10, #1226): what an `impl` hides is its value's form, the user's decision (A)

A function that gives a std adapter behind an `impl DoubleEndedIterator`,
`fn doubled(g) -> impl DoubleEndedIterator { g.iter().map(|x| x * 2) }`,
makes it std's compiled struct, as one held is (ADR 0871). Yet the type it
hid went into the crate's types made knowing their back (`back_stepped`'s
`typed`), and each of its callers' code read every value of it as the
knowing JS form. `groups.iter().flat_map(doubled).rev()` threw `it.back is
not a function`. `down(4).rfold(..)` of `fn down(n) -> impl
DoubleEndedIterator { (0..n).rev() }` made the unrelated `(1..6u32).rev()`
in `main` the knowing form, whose `last()` then read what wasn't there.

**User decision (A): value-owned, compiled wins.** Case C
([0262](0262-when-rust-and-js-disagree.md)): results changed.

- **Std's compiled struct a function makes behind an `impl` isn't a type
  made knowing its back** (`compiled_behind_an_impl`): `HELD_COMPILED`'s of
  this build, of what the `impl` hides in the function's own terms, so not
  its type parameters' values, which its callers make. Nor is an adapter
  made elsewhere that holds one (`holds_compiled`). Its form is its value's,
  as a named type's is. What steps such a value from its back is refused
  where a named one is, naming #1235, which lifts it through std's compiled
  `next_back`. An adapter the crate's makers do make knowing, `Filter` in
  the default build, or a `map` made in place of an `impl FnMut`, is as
  before.
- What a generic function gives back of its own type parameter behind an
  `impl` is the value its caller gave it, 23c0ebf42's (#1230).

`test/corpus/impl_trait_adapter_is_its_value.rs` holds it against native
Rust in both builds. `test/diagnostics.test.ts` holds the refusal.

## Amendment: `$rev` passes on its source's bounds again (#1226)

d4af55baf made a `rev()`'s knowing form its value's fact and dropped
`$rev`'s `of`, which nothing read then. A type that generic code steps
from its back is still on `BackStepped::typed`, though: each instance its
callers give, through an `impl` that gives its value as it is too. So in a
crate whose generic code `rfold`s `(1..6u32).rev()`, every `rev()` of a
`Range<u32>` is made `$rev($iterator(range, ..))`, and that range's own
`count()`, `last()` and `nth()` of another of them read its bounds, `.of`,
from the `$rev`. They threw `Cannot read properties of undefined (reading
'start')` where Rust gives 5. A wrong answer, so Rust's behavior is kept
(ADR [0262](0262-when-rust-and-js-disagree.md)'s third case).

`$rev` passes on its source's `of` again, as #721 had it. The bounds are a
runtime property of what a range's iterator steps, so they're read the same
however the value was made. A compile-time read through the `$rev` misses
an `nth()` whose `&mut` receiver drops the `$rev` mark.
`test/corpus/rev_range_counted_beside_a_generic_rfold.rs` holds it against
native Rust in both builds, and the mutation `rev-bounds-unpassed` holds
the line. This also fixes the metamorphic rows `opaque
rev_range_counted_beside_its_rfold.rs` and `generic-instances
range_inclusive_exhausted.rs`.

## Amendment: what a library's generic function is given, of a source JS's `filter` can't step from its back (#1109)

Since #1236, every iterator a crate gives a library's function is made
knowing its hint where it steps an array's items (`hint_asked.rs`), which
makes a slice's filter knowing its back too, so the two-crate case's
`back(v.iter().copied().filter(..))` no longer reached `library_backs` of
the caller, and mutation `library-given-backs-unmade` survived main's
check. The rule still decides what that one doesn't: a filter of the
library's own iterator, `back(Down { n: 4 }.filter(..))`, and of a
`VecDeque`'s items, which are no array's, are JS's own `.filter` without
it, whose `next_back` the library's body calls. ADR 0262's case: none, no
JS changes. The two-crate case gives `back` both, against native Rust,
and the mutation is caught again.

## Amendment: a flatten's items std's structs, stepped from its back (#1235)

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted;
what runs is std's.

A flatten stepped from its back makes each item what `each` gives knowing
its back. It read that from `each` as an arrow of one `return` alone. So
`flat_map(pairs).rev()`, of a function given by its name, `pairs` or
`Doubled::of`, whose `impl DoubleEndedIterator` is std's struct, was
refused, naming #1235. So was a closure that did more first, `|n|
owned(v(n))`, as a `Vec` collected in its body. Now `each_returned` reads
an arrow's return after statements that hold no others, and a function
given by its name as its call of a fresh item, `(item) => pairs(item)`.
Each item is made knowing its back by its compiled `next_back`, as a
closure's was. `test/corpus/flat_map_of_given_adapters_from_its_back.rs`
holds these and each other adapter of one a function gives, stepped from
its back, against native Rust in both builds. Still refused, naming
#1235: a function computed where it's given, `flat_map(pick())`, which
runs once, where `each` would call it for each item. Mutations
`flatten-each-by-name-unknowing` and `flatten-each-with-a-body-unknowing`.

## Amendment (2026-11-11, #1394): a zip a capacity-reading crate grows a `Vec` by is made knowing its length

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results; a
refusal lifted. A crate that reads a `Vec`'s capacity grows one by an
exact-sized iterator that isn't `TrustedLen` knowing its length (#1200), so
it asks that length (`as_exact_size`). A zip of an `inspect` is one, and a
zip of items with a destructor is held by MIR in a temporary. Of a held
zip, `knows_its_length` knew the length only where the crate stepped it
from its back (`$zipEnds`), so the plain build refused it, "can't count".
The neighbour shuffle found it beside a case that reads a capacity.

`stepped_locals` now marks such a zip's temporary as given to what asks
it. This applies in a crate that keeps capacities, where `vec_grown_by` says
a `collect()` or an `extend` grows a `Vec` by it, exact-sized and not
`TrustedLen`, as `grown_ask`'s `exact` asks. Its maker then makes it
`$zipEnds`, the one form that knows a zip's length, and its sides knowing
their backs, as a zip's held sides already are. An `inspect` so made is
`$mapEnds`, which keeps its source's length as a map's does, so it joins
`length_kept_source`.

Held by corpus case `zip_of_an_inspect_counted_where_capacity_is_read.rs`
(a collect and an extend, of items with a destructor), against native Rust
in both builds. The neighbour pair under #1394 runs as native Rust does,
and is taken off test/neighbours-known.txt. Mutations
`grown-zip-length-unasked` and `inspect-length-unkept` are new; three of
`length_kept_source`'s are refitted.

## Amendment (2026-11-12, #1406): a map stepped from its back where it's made, of a type parameter's items or giving each as it is

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results; a
refusal lifted. `v.into_iter().map(|x| x).next_back()` was refused, "`next_back()`
of a lazy or generic `std::iter::Map`, from its MIR". So was
`v.into_iter().map(|x| (x, 1)).next_back()` of a `Vec<T>` whose `T` a
caller gives a value with a destructor. MIR holds the map in a temporary
before its `next_back`, and two things were lost there:

- **A map giving each item as it is** is its source, marked so
  (`mapped_as_it_is`, #1342). `IterLocals::maps`, which keeps a
  temporary's map of an array's items for the `next_back` that reads it,
  now keeps one of these too. `next_back_made_here` steps its source made
  knowing its back (`as_double_ended`): `$nextBack($iter(v))`, in generic
  code and not.
- **A map of a type parameter's items whose caller gives it a drop**
  (`drop_params`', ADR 0098) is closed where it's made, as one of items
  with a destructor is. A closed source has no form that steps it from its
  back. `stepped_locals` asked `drops`, which counts no type parameter's,
  and `dropping_items` asked the item of `vec::IntoIter<T>`, which the
  crate's typing environment can't normalize. Now a receiver of
  `next_back` whose items are its function's own type parameter, given a
  drop (`given_a_drop`, or any of a library's, whose callers it can't
  see), is marked as stepped from its back. An owning array's source gives
  its item as its first type. Its maker makes it as the concrete one is,
  `$held($mapEnds($closing($iter(v), drop), f))`, the rest dropped as the
  temporary is. Where no caller gives a drop, the form is the plain one,
  `$nextBack($iter(v))`, its item mapped.

Held by corpus case `map_as_it_is_stepped_from_its_back.rs` (generic and
not, items with a destructor, `Option`s, a clone, a real closure), against
native Rust in both builds.

## Amendment (2026-11-12, #1235): a flatten's function value, given as it is, its items made knowing their backs apart

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results; a
refusal lifted. The last shape #1235 refused was a `flat_map` stepped
from its back whose function is computed where it's given,
`flat_map(pick())`, a `fn` pointer, an `impl Fn` or a `Box<dyn Fn>`.
Each item was made knowing its back by rewriting `each`,
`(item) => pairs(item)`, which would call `pick()` again for each item
where Rust calls it once.

Now a flatten's `each` is one of two kinds, each with one form:

- **An arrow written here** is rewritten. Each `return` of its own, a
  branch's too, not a closure's in it, is made knowing its back where
  it's made: its form decides that, `$mapEnds` of a map made in place.
  Before, only an arrow ending in its one `return`, after statements
  that hold no others, was rewritten. So a closure that returned from a
  `match` arm's block was refused.
- **Any other function value**, given by its name, `flat_map(pairs)` or
  `Doubled.of`, or computed, `flat_map(pick())`, is given as it is. Its
  items are made knowing their backs by their type, std's struct of an
  adapter a function gives (#1226), in `$flattenEnds`' third argument:
  `$flattenEnds(src, pick(), (items) => $knowing(items, ..))`. JS
  evaluates the arguments in order, so `pick()` runs once, after the
  source, as Rust's does. The runtime composes `knowing` into `each` once,
  for its steps, its `fold` and `rfold`, and its `count`.

The refusal's follow-up, "which holds std's struct of an adapter a function
gives (#1235)", is gone with `back_refused_because`, and so are its
diagnostics row and mutation. No shape tried reaches it now: a returned
map's `peekable()`, `filter`, `chain`, `zip`, `skip`, `step_by`,
`enumerate`, a `fuse()` of a returned filter, a flatten of returned maps,
each `rev()`'d or `next_back()`'d, all run as native Rust does in both
builds. A refusal of what can't be stepped from its back is the plain one.

Held by `test/corpus/flat_map_of_given_adapters_from_its_back.rs`, now
with a `fn` pointer, an `impl Fn` and a `Box<dyn Fn>` computed after a
source that prints, closures returning from an `if` and from a `match`
arm's block, `rev()`, `next_back`, `nth_back` and `rfold`, against native
Rust in both builds. Mutations `flatten-ends-knowing-unapplied`,
`flatten-ends-fold-knowing-unapplied` and
`flatten-each-branch-returns-unknowing` are new;
`flatten-each-by-name-unknowing`, `flatten-of-iterators-unknowing` and
`flatten-ends-made-unstaged` are refitted.
