# 0899. A `Flatten` held is std's, and an associated type's `IntoIterator` is held as it is

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. Amends
[0871](0871-rev-is-stds.md) (its `Flatten` paragraph), extends
[0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md),
[0543](0543-a-held-peekable-keeps-what-it-peeked-at.md) and
[0145](0145-type-facts.md). Fixes #278. The review's P1, its fifth
adapter.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A
held flatten's `size_hint` was `(0, None)` where Rust's counts, a
flatten of `Option`s stepped from its back threw, and generic code given
`Option` items as an `I::Item: IntoIterator` threw (#278).

## Context

ADR 0871 compiles `Rev`, `Enumerate`, `Map` and `Inspect` from std's
bodies where they're held, and listed what `Flatten` needs. On main, a
held flatten was the recognizers' `flatMap`, which can't answer
`size_hint`; `[[1, 2, 3], [4, 5, 6]].into_iter().flatten().size_hint()`
was `(0, None)`, Rust's `(6, Some(6))`, and `vec![Some(1), None,
Some(3)].into_iter().flatten()` stepped from its back threw `it.back is
not a function`. Compiling std's `Flatten` showed four more things, each
its own wrong answer or refusal:

- `FlattenCompat`'s `OneShot` specialization is not only an
  optimization, as the first attempt assumed: its `size_hint` of
  `Option` items is `(0, upper)` where the default's is `(0, None)`.
- `size_hint` asks `<I::Item as ConstSizeIntoIterator>::size()`, `N` of
  an array's items, which only the caller knows.
- `FlattenCompat` steps `I::Item`'s `into_iter()` by a generic
  `$intoIter`, which an `Option` (`T | undefined` in JS) isn't. The same
  gap was on main in the crate's own generic code: `for x in item` of an
  `item: I::Item` threw for `Option` items (#278).
- An adapter a `let` holds and steps itself, `let mut m = v.iter().map(f);
  m.next(); m.size_hint()`, was the recognizers' JS iterator, whose hint
  is `(0, None)`, also on main.

## Decision

**`flatten()` held is std's `Flatten`, `{ inner: { iter, frontiter,
backiter } }`, stepped by std's compiled `next`, `size_hint`, `fold`,
`next_back` and `rfold`.** One made in place stays the recognizers',
`$flattenEnds`; `Clone` and `Debug` of a held one stay refused, as they
were (`FlattenCompat`'s derives reach `Fuse`'s through a body rustc
inlined).

- **A specialization a fact picks is one rule, `fact_specialization`**
  (it was `fused_specialization`): the impl rustc selects and the one that
  specializes it on `FusedIterator` (`IFused`, ADR 0543) or `OneShot`
  (`UOneShot`), called by the caller's fact, `UOneShot ? oneShot(..) :
  general(..)`. Discovery compiles both impls' methods. `TrustedLen`
  alone stays an optimization, `optimized_default`.
- **`ConstSize` is a type fact** (ADR 0145): `N` of `[T; N]`, `&[T; N]`
  and `&mut [T; N]`, `undefined` otherwise; of a type only a caller knows,
  the caller's, refused where none gives it. What a callee asks of a
  slot, a type parameter's or an associated type's, `I::Item`'s, its
  caller asks of its own, a library's callee's as the crate's are
  (`asked_of_caller`, the one place that maps a callee's ask to its
  caller's slot; a library's was read by its type parameters alone).
- **An associated type bound by `IntoIterator`, `I::Item`, is held as it
  is (ADR 0552)**, as no call makes its values a JS iterator: generic code
  is given its dictionary, `IItemIntoIterator`, and steps what it makes by
  the `Iterator` that carries, `$given(IItemIntoIterator.intoIter(item),
  IItemIntoIterator.IntoIterIterator())`, given on, flattened or in a
  `for` loop. `I::IntoIter`, what this function's own `into_iter()`
  makes, is not one: it's the JS iterator it gives. A parameter a bound
  names its `IntoIter`, `U` of `IntoIter = U`, is the same value, given as
  it is too.
- **Every `IntoIterator` dictionary is one form, `{ intoIter,
  IntoIterIterator }`**, a reference's too (ADR 0457's), as generic code
  that holds what it makes steps it.
- **`size_hint()` keeps an adapter in place only where the recognizers
  size it as Rust does**, `HELD_COMPILED`'s third column: `rev()`,
  `enumerate()`, `map()` and `inspect()` made in place, `$sizeHint` of
  their source; not a flatten, nor any a `let` holds once it's stepped:
  those are std's struct, which knows its hint.
- What std's compiled bodies needed, each general: a closure rustc's MIR
  inlined from another function is read in that function's generics
  (`Mir::instantiated`, `inlined_closure_args`), its parameters passed on
  in the analyses; `cold_path`, `assume`, `likely` and `unlikely` are
  nothing and the value; `AddUnchecked`, `SubUnchecked` and `MulUnchecked`
  are `+`, `-` and `*`; a checked add's flag read as a value is
  `$overflowing`'s; `saturating_*` and `wrapping_*` are std's number ops;
  a discriminant read as a value is its number.

## Why

`std_flatten.rs` runs as native Rust does, every line, where main's
compiler printed `(0, None)` for each hint and threw at `next_back` of
`Option`s. `into_iter_of_items.rs` runs `Option`s, `Vec`s, arrays and the
crate's iterators through a `for` loop, a function taking any
`IntoIterator`, a closure's `into_iter().sum()` and `flatten()`, each as
native Rust does; on main, two of its four forms threw. The corpus passes.

## Consequences

- A held flatten's JS is std's, `iterator_flatten(..)` and
  `flattenDoubleEndedIterator_nextBack(it, ..)`, heavier than
  `$flattenEnds` was where that was right, a `let` stepped from its back
  among them (`flatten_from_its_back.js`), as a held `Rev`'s is.
- Generic code over `I::Item: IntoIterator` takes `IItemIntoIterator` and
  steps by it (`associated_into_iter.js`, `nested_item_drops.js`); a `for`
  loop of it is a `while` loop of its `next`, as ADR 0552's is. Making
  that `for (const x of $given(..))` is readability work left open.
- Every `IntoIterator` dictionary has `IntoIterIterator`
  (`into_iter_of_ref_param.js`).
- `let mapped = c.map(f); mapped.size_hint()` is std's `Map`
  (`adapter_size_hints.js`).
- The readability baseline is blessed with it: `$iter` 77.6 to 81.8 in
  10,000 lines, most of it the two new cases' sources, `vec![..].into_iter()`,
  the rest the dictionaries' `intoIter`; `Iterator.from` 14.7 to 14.8;
  `return;` 9.7 to 9.8, `Countdown`'s `next`.
- Open: `map(total)` of a function taking any `IntoIterator`, given an
  `Option`'s items, isn't converted (#281); `FlatMap` stays the
  recognizers'.
- Mutations: `one-shot-optimizing`, `alias-into-iter-not-held`, `size-hint-in-place`,
  `flatten-sized-in-place`, `held-into-iter-param-not-given`,
  `specialized-facts-unfollowed`, `one-shot-never`,
  `flatten-items-not-converted`, `ref-into-iter-steps-missing`,
  `in-place-flatten-unmarked`.

## Amendment: `OneShot` left out of the optimizations decides nothing now (2026-10-28)

The mutation `one-shot-optimizing`, `OneShot` added to `OPTIMIZING`, so
that `optimized_default` would resolve `FlattenCompat`'s specialized
methods to the general impl's, survived main's check. Since 7164b02aa
(#380), a call whose impl a fact picks asks `fact_specialization` first,
and takes its facts where rustc's resolution is none or the selected
impl's own method, which `optimized_default` would give
(`traits/calls.rs`); `resolves_to_crate` answers the same of either
(`items.rs`). Only std's generic bodies call `FlattenCompat`'s methods
unresolved. With the mutation, `test/std-package.test.ts` (the committed
`@rust-js/std` compiled again), `test/iterator-forms.test.ts`, the 75
corpus cases of `flatten`, `flat_map` or `size_hint` with their
snapshots, and the lowering-iterators, collections, options, diagnostics,
tidy-stages, snapshots and js-builtins suites all pass: it changes no JS.
The mutation goes; `OPTIMIZING`'s comment still says why `OneShot` isn't
one, as std's `OneShot` impls change what `size_hint` gives.

## Amendment: a zip whose hint is asked is std's

`v.iter().zip(v.iter().skip(2)).size_hint()` was `(0, None)` where Rust's
is `(3, Some(3))`, and `(0..4).zip(..).rev()`'s was too (#400): a zip
was listed sized in place, but what it's made of in place, `drop(2)`,
`map(f)` or an owned `Vec`, is JS's own iterator, which counts nothing,
so `$lazyZip` couldn't say. **A zip isn't sized in place**
(`HELD_COMPILED`'s `Zip`): one whose `size_hint()` is asked is std's
struct, whose compiled `size_hint` asks each side's, std's too. **A
`rev()` or another sized one is sized in place only where what it's made
of is** (`in_place_adapters`' `sized`), so a zip's `rev()` is std's too.

What's asked of the sides then is std's: a range's, `$rangeHintOf`'s,
which `$sizeHint` now asks of a range given as one, its exact count and
`n..`'s `(usize::MAX, None)`, std's `RangeFrom`'s, of any type. A zip
made in place where its hint isn't asked, given to generic code, hints
its sides' as std's `Zip` does, each what its JS value tells
(`$zipHint`), as a generic iterator's hint is (`semantics.md`, 14).

Each such zip is std's struct of its sides, an array's items a `$iter`,
`iterator_zip($iter(v), $iter(w), ..)`, so `docs/readability.txt`'s
`$iter` share grew with the corpus case below (81.6 to 83.1 in 10,000
lines), blessed as this decision's cost.

Case R: the hint is std's, where it printed another. Held by
`test/corpus/zip_size_hints.rs` against native Rust: zips of an array's
items, a `skip`, a slice, an owned `Vec`, ranges, a `map`, an endless
range, a `rev()`, of an array's items held, given to generic code, and
held and stepped. Mutations `zip-sized-in-place` (`recognition/std_items.ts`),
`rev-sized-whatever-its-source` (`body_queries.ts`), `range-from-unhinted`
(`runtime/range_len.ts`), `size-hint-range-given-unasked`
(`runtime/size_hint.ts`), and `lazy-zip-unhinted` and `zip-hint-*`
(`runtime/lazy_zip.ts`).

## Amendment: a constant size is asked only of what may be an array (2026-11-06, #813)

Case ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted; N.

Std's `ConstSizeIntoIterator::size()` is `N` of `[T; N]`, `&[T; N]` and
`&mut [T; N]`, and none of any other type. Generic code's
`rows.iter().flatten()` of a `&[Vec<T>]`, std's struct in the build of
std's structs, was refused, "its constant size of `&Vec<T>`, which no
caller gives": every type that held a type parameter was taken for one a
caller answers. Now only what may be an array, a type parameter or an
alias through references, is; any other has none whatever its
parameters; and an array's is its length, of a const parameter its value
as the function's given it (`const_arg`), `[T; N]`'s `N`.

`test/corpus/generic_flatten_of_vecs.rs`, of `Vec`s and of arrays of a
const length, collected and asked their hint, against native Rust in
both builds, and in `iterator-forms.test.ts`' few. Mutations:
`const-size-of-built-type-asked`, `const-size-of-a-const-length-none`.

## Amendment: a reference's constant size is its pointee's (2026-11-11, #1273)

Case ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted; no
observable difference to decide.

Generic code given `r: &'a I` bounded `&'a I: IntoIterator`, flattening
it, `[0, 1].iter().flat_map(|_| r)`, was refused in both builds: "its
constant size of `&I`, which no caller gives". A caller answers facts
of a type parameter, `I`, and `&I` had no answer of its own. std gives
`N` for a `[T; N]`, a `&[T; N]` and a `&mut [T; N]` alike, and
`&&[T; N]` is no `IntoIterator`, so `&I`'s and `&mut I`'s constant size
is `I`'s whatever `I` is. `held_facts`, which already says a `&mut I`'s
`TrustedLen` is `I`'s (ADR 0975), now says so of a constant size of a
reference to a type only a caller knows. Both the analysis, which asks
the caller (`asked_of_caller`), and the lowering (`type_fact_value`)
read it there, so the function is given `IConstSize`: `3` from a
caller's `&[char; 3]`, nothing from a `&Vec` or a newtype's `&Word`.

`test/corpus/flat_map_of_a_bound_reference.rs` steps and collects `&I`
and `&mut I` flattened, asks each hint as it goes, and passes a reference
on through another generic function, against native Rust in both
builds. Mutation: `ref-const-size-not-its-pointees`. A
`size_hint()` of `once(i).flatten()` or `once(r).flat_map(..)` stays
refused, of a plain `I: IntoIterator` as well: `Once`'s hint of what
it holds, a separate gap.

## Amendment: a flatten given where its hint is asked is held (2026-11-11, #1323)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

A flatten made in place is JS's own `flatMap`, which knows none of its
items' sizes, so its `size_hint()` read `(0, None)` until its source ran
out. Asked where it's made, `it.size_hint()`, the flatten was held, std's
struct, which counts an array's constant size and an `Option`'s one at
most (above). Given to a function that asks its hint,
`hint(v.iter().copied().flat_map(|n| [n; 3]))` of `fn hint<I: Iterator>(it:
I) -> (usize, Option<usize>) { it.size_hint() }`, it was made in place.
That gave `(0, None)` where native Rust gives `(12, Some(12))`, and `(0,
None)` where it gives `(0, Some(4))` of `flat_map(Some)`. So did one
given on, `relayed(it)` of `hint(it)`, and one under another adapter
made in place there, `flat_map(f).map(g)`, `.filter(g)` and `.rev()`,
whose hint is their source's. The plain build alone was wrong; in the
build of std's iterator structs a flatten is always std's.

`in_place_adapters` holds a flatten whose hint is asked where it's given:
- given to `size_hint()`, to a function of the crate's that asks the hint
  of what it's given, or to a library's, whose body the crate can't see,
  as `hint_asked.rs` takes a library's already;
- or as the receiver of an iterator's method whose own call is wanted.

Those calls are `wanted`, found to a fixed point of the body's calls.
Which functions ask the hint of what they're given is the hint's owner's,
`hint_asked.rs`' `asked_of_own` (the generic facts' catalogue's row), read
across the crate's bodies to a fixed point: `size_hint()` of a type made
of the function's type parameters, or one given on to such a function. It
goes by the function, not its parameter, as what it's given may reach the
hint through what it makes of it, `metamorph_is(it)`'s `I` of an `impl
MetamorphIs<I>`; so each iterator it's given is wanted, which holds a
flatten where nothing asks, std's struct, correct and longer. The bodies
that give one such a function or a library's make what they give in
place again, `results_in_place`. The other adapters given there are as they were,
`$lazyZip(..)`, `$stepBy(..)`, `$filterEnds(..)`: JS's own and the
runtime's know their hints.

Left as it was: generic code's flatten of a trait's associated type's
items, `(1..3).flat_map(S::items)`, in the plain build, is now held, and
asks the constant size of `<S as Named>::Items`, which no dictionary
carries yet. That was a wrong answer and is now refused, as it is in the
build of std's structs, until #1312's `Asked::Carried` carries it.

`test/corpus/flatten_given_where_its_hint_is_asked.rs` holds a flatten
of arrays, of `Option`s and of a map's arrays given to `hint`, one given
on, one through `identity`, one under a `map`, a `filter` and a `rev()`, one asked where it's
made of a type parameter's items, and the other adapters given there,
against native Rust in both builds. Mutations (`lower/body_queries/in_place.ts`):
`flatten-wanted-made-in-place`, `hint-wanted-not-through-adapters`,
`hint-asked-bodies-unrevisited` and `hint-of-a-library-unasked`, and
(`lower/analysis/hint_asked.ts`) `hint-asked-of-no-param` and
`hint-asked-not-relayed`; `test/crates.test.ts`' "a consumer's flatten
given to a library's function knows its hint"; `rev-never-in-place`,
`rev-own-impl-in-place` and `holder-param-steps-js` refitted.

## Amendment: held where a collect's hint is seen, decided once (2026-11-11, #1327)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

A `flat_map` of arrays collected into a `Vec`, or extended into a `Vec`
or a `VecDeque`, asked its source's `size_hint` three times where std asks
once, when that source was the crate's own iterator, whose hint prints.
std's collect asks the flatten's hint as the `Vec` fills. std's flatten
knows its items' constant size, so its hint is exact and one reserve is
enough. A flatten made in place, JS's own `flatMap`, knows none, so the
`Vec` asked again as it grew. #1323 held a flatten whose hint a function
asks. A collect or an extend asks it too, and the program sees what it
asks wherever it reaches a `size_hint` of the crate's own or a
library's. So a flatten given one there is held:
`iterators/hint_asks.rs`' `collect_hint_seen`, the owner of when a
collect asks its hint, seeds `in_place_adapters`' `wanted`. A capacity
the crate reads is std's already, by the growth that #1180 keeps.
Without this rule, four capacity-reading flattens match native Rust, so
it holds none for them, and a flatten whose hint runs nothing is made in
place as it was.

Held, std's struct asks its items' constant size, which a generic
function's callers give only of a type parameter or one of its slots,
`I::Item`. Of another type only a caller knows, `<S as Source>::Iter` or
a trait method's `impl Trait` (#1312), none gives it yet, so holding the
flatten would refuse a program that compiles today. The user's decision,
through m6: no new refusal. `in_place.rs`' `flattens_sized` holds a
flatten for a seen hint only where each flatten in what the collect is
given has its size given, `analysis/type_facts.rs`' `const_size_given`
says, the owner of what a caller is asked. Another stays made in place,
as it was. Where a caller makes that type an array whose constant size
std counts, its hint is asked again as the `Vec` grows, which native Rust
doesn't: #1341 (P0), already so before this change, closed by carrying
`ConstSize` through the dictionary with #1312. An inner that isn't an
array, the crate's own iterator say, matches native Rust in place: std
counts no constant size of it.

Which functions' hint is seen is a `Recognition` question, which needs
the crate's libraries and trait impls. So the in-place adapters are no
longer decided as the bodies are collected. `body_queries::InPlace` is
undecided until then, and its readers ask only `contains`, which an
undecided one refuses. The set is decided once by its owner,
`body_queries/in_place.rs`' `decide`, which `lower/pipeline.rs` calls
after the crate's ground (`analysis/ground.rs`: libraries, trait impls,
cloned types, capacity facts) and before the analysis lends the bodies
out. Initializers are decided too, and so is the build of std's iterator
structs, whose set is empty. The first pass `collect_bodies` made, and
`results_in_place`'s second, are now this one pass: each body with the
crate's facts, then each `flat_map`'s closure made in place given that
answer. `test/architecture-iterators.test.ts`' "in-place adapters are
decided once, then read" holds it, and docs/architecture.md's pipeline
shows the step. A corpus compile of all 2,161 cases, 12 at a time on the
Mac, took 15.7–16.3 s before and 15.9–16.3 s after.

`test/corpus/flatten_collected_whose_own_hint_runs.rs` and
`flatten_collected_where_its_hint_is_seen.rs` (a collect, a `Vec`'s and a
`VecDeque`'s extend of the crate's iterator, and a plain range's collect
whose capacity is read) run against native Rust in both builds. Three
snapshots changed: `collect_asks_hint_as_it_grows` and
`flat_map_of_a_where_clause_into_iterator` hold their flattens now, which
reach the crate's own `size_hint` and a caller's, and
`restarting_iterators` the one it gives generic code that collects it. #1327's 60 rows are off
`test/iterator-matrix-known.txt`, and the whole conformance and points
tables were run. `flatten_of_its_callers_items_where_its_hint_is_seen.rs`
holds a generic flatten of a slot's items, and
`test/lowering-generic-iterators.test.ts` a trait's associated
iterators made in place. Its metamorphic net's generic-instances rewrites
found that a flatten given to generic code that collects it was made in
place, though #1239 made that collect std's `SpecFromIter`, which asks its
hint as the `Vec` grows: `hint_asked.rs`' `AsksHint` is now the one
answer of which calls ask the hint of their parameters' iterator, of
what's made knowing it (`of_crate`) and of which functions ask it of what
they're given (`asked_of_own`), so the flatten is held, and the third case
holds that too. The second case's rewrite is listed under #1263, the
capacity generic code's collect gives, in both builds. The readability baseline is blessed with it: `$iter` 89.8
to 89.9 in 10,000 lines, the held flatten's dictionary's `intoIter:
(items) => $iter(items)`. Mutations: `collect-hint-seen-made-in-place`,
`collect-hint-seen-unsized-held` and `flattens-sized-unwalked`
(`lower/body_queries/in_place.ts`), `collect-hint-seen-of-none`
(`lower/iterators/hint_asks.ts`), and `const-size-slot-ungiven` and
`const-size-of-known-ungiven` (`lower/analysis/type_facts.ts`), and
`collect-asks-no-hint` (`lower/analysis/hint_asked.ts`). `std-iterators-ignored`,
`growing-extend-capacity-unkept` and `held-flat-map-results-in-place`
were moved or refitted, and `hint-asked-bodies-unrevisited` was taken
off: the pass it named is gone.

## Amendment: its items' constant size, carried by their trait's dictionaries (2026-11-11, #1312, #1341)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

#1327 held a flatten for a seen hint only where its callers give its
items' constant size, a type parameter's or one of its slots', `I::Item`.
Of another associated type, `<S as Source>::Items`, none was given, so
generic code's `Own(4).flat_map(|n| S::items(n)).collect()` stayed made in
place: where a caller made `Items` an array, std counts its constant size
and asks the hint once, but the `Vec` asked it again as it grew (#1341).
And the build of std's structs, which holds every flatten, refused one of
a trait method's `impl Trait`, `flat_map(S::hidden)`, as no dictionary
carried its facts (#1312).

What a std callee asks of an associated type no slot names is now what
its trait's dictionaries carry (`type_facts.rs`' `asked_of`,
`Asked::Carried`), as a fact the crate's code asks of `<A as
Array>::Item` already was (ADR 0145). Each impl's dictionary carries it
of its own type, `ItemsConstSize: 2` of an array, and generic code reads
it, `SSource.ItemsConstSize`; a trait method's `impl Trait`'s is named
by its method, `HiddenConstSize` (the user's decision through m6, as its
`$dropHidden` is, #1296). So is any fact a std callee asks so, how
generic code's collect of `S::Iter` into a `Vec` collects it, its
`IterVecCollect` (#1239): two snapshots, `associated_iterator_given_as_it_is`
and `associated_iterator_dropped`, carry it now, `"inPlace"` of a `Vec`'s
iterator, where the collect took it for one it steps by `next`. An entry that's `undefined`, a `Range`'s or a
`Map`'s constant size, which is none, is left out. `const_size_given`
says it's given where the trait's dictionaries carry it, of the crate's
trait or a library's whose manifest says so, so `flattens_sized` holds
these flattens. A library's trait whose manifest doesn't say so is made
in place as before, which `test/crates/pairs/library_trait_items_flattened_where_its_hint_is_seen`
holds, of a `Range`'s items, which have no constant size: where a
consumer's own impl makes them an array, its hint is asked again as the
`Vec` grows, which stays #1341's. The crate's own callee given `A::Item`,
whose ask its caller's dictionary carried by a list of its own
(`passed_carried`), is that relay too now, its list gone. `generic.rs`' refusal of a trait method's `impl
Trait`'s facts by name goes: what's left is the refusal of a fact no
caller gives.

`test/corpus/flatten_of_a_traits_items_sized_by_its_dictionary.rs` (an
associated `IntoIterator` an impl makes an array, an associated
`DoubleEndedIterator` and an `impl DoubleEndedIterator`, collected where
the crate's own hint prints and stepped from the back) runs against native
Rust in both builds, and `test/lowering-generic-iterators.test.ts` runs
#1292's and #1308's flattens in the build of std's structs too.
Mutations: `callee-fact-of-assoc-uncarried` and
`const-size-of-carried-ungiven` (`lower/analysis/type_facts.ts`);
`rpitit-fact-refusal-unnamed` and `assoc-fact-unpassed` are taken off with
what they held, three were refitted, and `collect-hint-seen-unsized-held`
and `flattens-sized-unwalked` are held by the crate pair now. An `impl IntoIterator` alone, whose `IntoIterator`
dictionary of the opaque type no evidence gives, is still refused in both
builds: #1345.

## Amendment: every trait carries its `IntoIterator`s' constant size (2026-11-11, #1341)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

A library's trait's dictionaries carried only what its manifest says its
own code asks. So a consumer's generic `Own(4).flat_map(|n| S::items(n))
.collect()` of dep's `trait Source { type Items: IntoIterator; .. }`,
given its own `impl Source for Pairs { type Items = [u32; 2]; .. }`, found
no constant size given and stayed made in place: std counts the array's
and asks the crate's own printing hint once, but the `Vec` asked it again
as it grew. A consumer can't add to a library's dictionaries, so the
user chose, through m6: every trait's dictionaries carry the constant
size of each of its associated types that's an `IntoIterator`, whatever
asks it, and a library's manifest says so (`type_facts/carried.rs`'
`carried_by_every_impl`). Only an array has one, so an impl of anything
else carries `undefined`, which is left out, and the output grows only
where an impl gives an array. Not of one its bounds make an `Iterator`
(`recognition::const_size_unknown`): no array is one, so its constant
size is none, and carrying it would make an impl that gives its own `T:
Iterator` ask its callers `T`'s, which generic code doesn't give
(`iterator_passed_through_a_chain_of_impls.rs`).

With that, every flatten's items' constant size is given wherever it's
held, the crate's own traits' and a library's alike, so #1327's guard,
`flattens_sized` and `const_size_given`, is gone: a flatten a collect sees
the hint of is held, std's struct.

The manifest's `associated` entries now list `ConstSize` of each such
type, which a consumer reads as the library's dictionaries carrying it, so
the library ABI is 28 (`src/library.rs`' `VERSION`,
`tooling/manifest.js`): a library built before carries none, and a
consumer would read `undefined` where an impl gives an array.

`test/crates/pairs/library_trait_items_flattened_where_its_hint_is_seen`
(dep's trait and its `Range` impl, the consumer's array impl, each
collected, and mapped and collected, where the consumer's own hint prints)
runs against native Rust. Mutations: `into-iterator-sizes-uncarried` and
`iterator-assoc-sizes-carried` (`lower/analysis/type_facts/carried.ts`),
`iterator-const-size-unknown` (`lower/recognition/type_facts.ts`); the
guard's five went with it.
