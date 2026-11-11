# 0510. `flat_map` and `flatten` of what has a destructor

Status: Accepted. Extends [0465](0465-dropping-an-adapter-is-closing-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
in what order, would change, so Rust's is kept, paid for in output.

## Context

`flat_map` and `flatten` of a source of items with a destructor were
refused. One whose inner iterators owned such items, `(0..3).flat_map(|i|
vec![Loud(i), ..]).take(1)`, compiled, and never dropped what the inner
one hadn't given. And finding them found more:

- **Two ways to lower `flat_map`**: `IterMethod::FlatMap`, `it.flatMap(f)`,
  which every call took, and `IterComb::FlatMap`, never reached, which
  alone knew a function that gives an `Option`. `flat_map(|x| if .. {
  Some(x) } else { None })` threw: JS's `flatMap` takes no number.
- **`Iterator.from(list).take(n).toArray()` was `Array.from(list).slice(0,
  n)`**: `list` stepped to its end, so a `$closing` in it was never closed.
- **Two places worked out an iterator's `Item`**: `iterator_item`, and
  `owned_items`' own projection for a `Splice`.

## Decision

**`flat_map` and `flatten` are `IterComb`'s, one way, and each holds its
source and the inner iterator it steps, both closed to drop what they
hold:**

- The source, an owning one, is `$closing`, as `zip`'s and `chain`'s are
  (`holds_its_sources`).
- Each inner iterator that owns items with a destructor is closed too:
  `(i) => $closing($iter(..), drop)`, `closed_inner`'s, `f`'s own parameter
  kept where it's an arrow of one value.
- Where both hold something, JS's `flatMap`, which closes the inner one
  first, is `$flatMap(it, each)`, which closes the source first, then the
  inner one, as Rust drops a `FlatMap`'s fields.
- A `FlatMap` or `Flatten` held, its type's drop, is `closed_on_drop`'s, of
  its source and inner iterator: `closing_both` and `closing_inner`.
- `flat_map` of a function that gives an `Option` is its `Some`s,
  `it.map(f).filter(present)`; `IterMethod::FlatMap` is gone.

And a list that mentions `$closing` isn't listed by `collected`;
`Recognition::iterator_item` is the one place an iterator's `Item` is
worked out.

## Why

- **It's the same program**: `flat_map_drops` stops inner iterators, a
  source, and both, early, holds a `FlatMap` and drops it, and flattens
  `Option`s and `Vec`s, against native Rust.
- **One way**: the `flat_map` that every call took was the one that was
  wrong; now there is one.

## Consequences

- A function given to `flat_map` that holds a value with a destructor is
  refused, as `zip`'s and `chain`'s are: its `FlatMap` isn't wrapped to
  drop it.
- Mutations close the inner iterator first, close none, refuse a source,
  take `flat_map` of `Option`s for JS's, list a closing chain, and drop
  no `FlatMap` held.

## Amendment: `$flatMap` wherever what it closes or gives drops, and its `last` is std's (2026-11-04, #644, #645)

The iterator matrix (ADR 1090) found two drop orders this decision got
wrong, by ADR 0262's "results change" case, so Rust's is kept:

- **The close order (#644).** `$flatMap` was chosen only where
  `closing_drop` said the source itself holds what has a destructor. A
  source that's an adapter of one, `vec.into_iter().map(f).flatten()`, has
  no closing of its own, so JS's `flatMap` closed the inner iterator
  first: dropped part-way, it dropped `2` before `1` where Rust drops its
  source, then its front inner iterator. `$flatMap` is now chosen
  wherever its inner iterators close to drop what they hold, whatever its
  source: one with nothing to drop is closed first to no effect.
- **`last` (#645).** std's `FlattenCompat::last` is
  `iter_fold(None, |last, inner| inner.last().or(last))`: each inner
  iterator's own `last`, which drops what it passes, then the one kept
  before. `$lastDropping` stepped it by `next`, dropping each item as the
  next came. `$flatMap` gives `$lastDropping` its parts, `inners()`: the
  inner iterator it's in, its source unless it's ended, and `f`; each
  inner one's `last` is taken as std takes it, the one kept dropped first
  where one panics, then what's left of the source. So `$flatMap` is also
  chosen where its inner iterators hold nothing to drop but give items
  that have a destructor, the crate's own iterators of them. And its
  source is fused, as std's `Flatten` holds it: stepped no more once it's
  given `None`, and dropped then, as `Fuse` drops what it holds at its
  first `None`.
- **Its fold.** `$flatMap` folds as `$Steps`' `flatMap` does, std's
  `FlattenCompat::fold`: the rest of the inner iterator it's in, then its
  source's own fold of each one's, so a `count()` reaches the crate's own
  `fold`; and its `try_fold`, given by `$forward` (ADR 1089). Where a stage
  panics as it folds, the inner iterator being folded is dropped before
  the source, as the frames unwind, which is the order JS's `flatMap` had
  by chance; dropped whole, the source first. So a collect into a
  `String`, which std's `String::extend` steps by its fold, is
  `$foldToArray`, not `$collectDropping`, where a panic may stop it.
  `--std-iterators` still drops the source first there: its compiled
  `FlattenCompat::fold` leaves the inner iterator a panic stops to the
  struct's drop, which the matrix's known list keeps under #644's std
  cases.
- **std's build.** `Flatten::last` and `FlatMap::last` are compiled into
  `@rust-js/std`, so a held one, and every one in `--std-iterators`, asks
  std's own. That needed two things of compiled bodies: a function std
  keeps private given as a value, `FlattenCompat::last`'s own `last` given
  to `iter_fold`, is compiled with what's given it (`find_compiled_std`);
  and a closure optimized MIR names as a constant, as it captures nothing
  of a size, captures each function item it holds as that function's
  value, `filter_map_fold`'s of `into_item` (`operands.rs`). Such a
  function is called by what it's given to, so `instances::calls_of`
  counts each a closure constant holds as named: `Iterator::last`'s
  `some`, of an `Option`'s `T`, is given `T`'s drop, which it drops the
  one it replaces by. And a function std writes inside another is named
  after it, `flattenCompatIUIterator_last_last` and `iterator_last_some`:
  every crate's names are made around the package's exports, and a bare
  `last` or `some` there renamed a crate's own `some` to `some$1`.

Readability: a `flatten` of inner iterators that drop, of a source that
drops nothing, is `$flatMap($lazyRange(0, 3), (i) => ..)` where it was
`$lazyRange(0, 3).flatMap((i) => ..)`. `flatten_closed_part_way_drops_source_first`
and `flatten_last_takes_each_inner_last` hold both against native Rust,
in both builds. The mutations `flat-map-js-close-order` and
`flat-map-of-dropped-items-js` (`combinators/adapters.ts`),
`flat-map-helper-backless` (`iterators/knowing.ts`, a `$flatMap` stepped
from its back is `$flattenEnds` as `.flatMap` is), `flat-map-no-inners`,
`flat-map-source-unfused`, `flat-map-last-source-unfused`,
`flat-map-fold-by-next`, `flat-map-source-kept-at-its-end`,
`string-collected-by-next` (`mir/iter/collect.ts`),
`last-inners-default` and `last-inners-kept-on-panic` (`runtime/`), and
`named-private-fns-uncompiled`, `closure-constant-captures-none`,
`closure-constant-functions-uncalled` and `std-nested-fn-named-bare`
(`test/std-package.test.ts`) are caught.

`flat-map-closed-as-it-throws`, which closed the source inside `next`
where `f` panics, before a `last`'s kept item was dropped (#426), is gone
as an equivalent mutant: a flatten's `last` takes its parts now, and
drops its kept item itself before what's left of the source, and every
other consumer that steps it by `next` closes it as it throws, its rest
first, so closing the source earlier changes nothing it prints.

## Amendment: a flatten's `count` is each inner iterator's own (2026-11-05, #736)

std's `FlattenCompat::count` is `iter_fold(0, |acc, inner| acc +
inner.count())`: the own `count` of the inner iterator it's in, then of
each its source makes, then of the one at its back. Both builds counted
each inner iterator by `next`, so the crate's own `count` never ran: of
`vec![2, 3].into_iter().flat_map(Own).count()` native Rust prints `own
count 2`, `own count 3`, the JS printed neither. Results change, so
Rust's behavior is kept (ADR 0262):

- **One catalog says it** (ADR 1089): `OwnStep::Count` of a `Flatten` or
  a `FlatMap` asks `INNERS`, each inner iterator it makes,
  `<U as IntoIterator>::IntoIter` (`Recognition::flattened_inner`),
  not one of its type arguments. Where that reaches the crate's own
  `count`, or a type parameter's, the flatten made in place is
  `$flatMap`, as one of inner iterators that drop is, and a `count` of it
  is `$count`; one whose inners' `count` is std's stays JS's `flatMap`
  counted by `reduce`, byte for byte as before.
- **The runtime's flattens count as std's does.** `$flatMap` and
  `$flattenEnds` have a `count`, given by `$forward`: the front inner's
  `$count`, then its source's fold of each item's inner one's, then, of
  `$flattenEnds`, its back inner's, each given the items' drop. A panic
  in an inner `count` leaves what's left to what takes it, which closes it
  as it throws, as `count(self)` drops it: what it still holds, its source
  unless its fold has it, which closes itself. (A `$consumed` of it,
  closing it there too, was an equivalent mutant.) Each says its `count` is its own,
  `ownCount`, so a `skip()` of it asks it, as std's `Skip::count` asks
  its source's.
- **std's build.** `Flatten::count` and `FlatMap::count` are compiled into
  `@rust-js/std`, so a held one, and every one in `--std-iterators`, asks
  std's own: a held one's `count()` is `$count(counted)`, where it was
  `counted.reduce((n) => n + 1, 0)`, std's default by `next`
  (`held_flat_map_of_maps_from_its_back`). That needed two things of
  compiled bodies: a closure a
  constant's type holds, not only a constant of the closure's type,
  `Iterator::count`'s inlined into `FlattenCompat::count`'s `OneShot`
  impl, is collected (`foreign_bodies`); and an associated type whose
  bounds say it's another, `<FilterMap<Fuse<I>, F> as Iterator>::Item`
  of that closure read in its maker's terms, `<U as Iterator>::Item`, is
  dropped as that one, by the drop its caller gives (`drops/types.rs`,
  `drops/parts.rs`). The same reading makes `FlattenCompat`'s `OneShot`
  folds drop an inner iterator by `dropU`, the drop given for `U`, where
  they read `IItemIntoIterator.$dropIntoIter`, the same function. The walk
  of each constant's type finds what the walk of each local's type found,
  `sum`'s in `Filter::count` (#424), so that walk is gone, and its
  mutation, `inlined-closures-unfound`, which survived as an equivalent.

A `&mut` inner iterator's `count` is std's default, by `next`, as std's
`&mut I` passes only `nth` on (ADR 1089); `--std-iterators` refuses a
flatten of `&mut` of the crate's iterator for its `IntoIterator`
evidence, which is filed apart.

`flatten_count_takes_each_inner_count` holds each against native Rust in
both builds. Mutations: `flatten-count-asks-no-inner` and
`flatten-inners-are-its-source` (`recognition/own_steps.ts`),
`flat-map-own-count-js` (`combinators/adapters.ts`),
`flatten-count-by-reduce` and `flatten-count-always-own`
(`mir/iter.ts`), `flat-map-count-by-next`, `flat-map-count-skips-front`
and `flat-map-count-not-passed-on`
(`runtime/flat_map.ts`), `flatten-ends-count-by-next`,
`flatten-ends-count-skips-back` and `flatten-ends-count-not-passed-on`
(`runtime/back_steps.ts`), `flatten-count-uncompiled`
(`recognition/std_items.ts`), `projection-drop-unnormalized`,
`projection-dropped-unnormalized` (`drops/`) and
`closure-in-constant-type-unfound` (`analysis/foreign_bodies.ts`).

## Amendment: an inner iterator a where clause makes, by its dictionary (2026-11-06, #829)

A `flat_map` or a `flatten()` of items with a destructor, each closed as
it's made (`closed_inner`), stepped its closure's result as a JS iterator
whatever its type. Take an associated type that a where clause says is
`IntoIterator`, such as a blanket impl's default's `Self::Item:
IntoIterator` copied into its impl: the item is a `Vec`'s array, and
stepping it threw `it.next is not a function`. It's now made an iterator by
its dictionary's `into_iter` (`items_by_dictionary`), as `inner_iterable`
already did for items with nothing to drop (#278). ADR 0262: results change
(a throw where Rust runs), so Rust's is kept. Held by
`flat_map_of_a_where_clause_into_iterator.rs`, both builds; mutation
`closed-inner-not-by-dictionary`. A generic function's own `I::Item:
IntoIterator` of items with a destructor is still refused, as ADR 0190
says.

## Amended (2026-11-10, #1240): an `Option` inner owns its item

A `flatten()` of `Option`s, or a `flat_map()` whose function gives one, is
made as a filter of what isn't `None`. Given to generic code with its drop,
`last_of(v.into_iter().flatten())` of items with a destructor, it was given
std's `Flatten` struct's glue, `flatten.inner.iter.return()`, which that
form doesn't have: `Cannot read properties of undefined (reading 'iter')`,
where native Rust prints its last. What closing a flatten drops asked its
inner `IntoIterator` what it owns, `closing_inner`, which knew a `Vec`'s
and an array's items but not an `Option`'s, so it said the flatten isn't
closed on its drop. An `Option` owns the one item it gives, as its
`IntoIter` does; the flatten is closed, `it.return()`, as a map's and a
filter's are. Held by `flatten_of_options_given_to_generic_code.rs` and the
mutation `flattened-option-owns-nothing`; the `generic-instances` row of
`flatten_last_takes_each_inner_last.rs` runs as Rust does.

## Amended (2026-11-12, #1109): its dictionary's `into_iter`, by `as_js_iterator`

`closed_inner` no longer asks `items_by_dictionary` itself: `as_js_iterator`
gives an associated type's items its dictionary's `into_iter`, `$given(
SelfItemIntoIterator.intoIter(x), ..)`, the same JS, so the mutation
`closed-inner-not-by-dictionary` survived every case that reaches it, 54
of them, `flat_map_of_a_where_clause_into_iterator.rs` among them, JS and
results the same. One rule, `as_js_iterator`'s, makes each inner iterator
of an associated type, closed or not.
