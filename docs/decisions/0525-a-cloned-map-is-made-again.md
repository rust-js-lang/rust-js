# 0525. A cloned `map` or `filter` is made again

Status: Accepted. Extends [0522](0522-a-closure-cloned-apart-is-made-again.md)
and [0181](0181-iterator-clones.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).
Amended by [0984](0984-a-map-or-filter-cloned-apart-keeps-its-back.md): one
stepped from its back is made knowing it.
Amended by [0999](0999-a-compiled-copied-is-cloned-by-its-struct.md): a
`copied()` std compiles held is cloned by its struct.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a clone of an iterator
is where the iterator is, with a clone of its closure, as Rust's `Clone` of
a `Map` is; nothing reruns.

## Context

A lazy adapter is a JS iterator helper, `it.map(f)`, whose state hides in
the engine: a clone of one was refused ("cloning
`std::iter::Map<std::ops::Range<i32>, {closure}>`"). A `map` or a `filter`
holds nothing but its source and its function, though: a JS helper's
position is its source's. A clone is the helper made again of a clone of
each, as a closure cloned apart is (ADR 0522).

## Decision

**A `map` or a `filter` the crate clones is made by a function of its
source and its function, `makeMap`, and carries its clone,
`$cloning(it.map(f), () => makeMap(clone(it), clone(f)))`; `$clone(it)`
makes it again.**

```js
const makeMap = (it, f) =>
  $cloning(
    $iterator(it, $rangeNext).map((item) => f(item)),
    () => makeMap({ start: it.start, end: it.end }, $clone(f)),
  );
const it = makeMap({ start: 1, end: 4 }, counter);
const copy = $clone(it);
```

- **One owner of what's cloned**: `analysis::clones`, the types each call's
  `Clone` bounds are given, revealed through `impl Clone`, and what each
  clone clones in turn, its `Clone` impl's bounds, to a fixed point:
  `Map<I, F>: Clone` clones `I` and `F`. Closures read their part of it
  (ADR 0522).
- **One question, `iterator_cloned_apart`**: a `Map` or `Filter` of the
  cloned types, where a generic function's `Map<I, {closure}>` stands for
  each its callers clone; what's made and what's cloned both ask it.
- **Its source is cloned as it is**: an array's iterator's, a range's,
  another made again; a `copied()` is the iterator it copies (ADR 0480).
- **A closure that writes nothing it took is shared**: `clone_shares` asks
  what its captures do, not its kind, which a `map`'s bound makes `FnMut`.

Still refused, in [the limitations](../guide/limitations.md): a clone of one that
keeps a count of its own, an `enumerate`'s, a `take`'s or a `skip`'s, whose
JS helper hides it; one of items with a destructor, or lent; and one
generic code makes where its source isn't bound by `Clone`, which it
couldn't clone.

## Why

`lazy_iterator_clone` clones a `map` of a closure that counts its calls,
partly stepped, a `filter` of a `map`, one through a `T: Clone`, one a
function gives as an `impl Iterator + Clone`, and one a derived `Clone`'s
field holds, against native Rust.

## Consequences

- `docs/guide/limitations.md` loses "A clone of a lazy iterator", and gains its
  narrower remains.
- Mutations stop following impl bounds, revealing an `impl Clone`, or
  matching generic code's closure; share a source or a closure; and make
  every `map` cloned apart where a type parameter is cloned.

## Amendment: an `enumerate`, a `take` and a `skip`

An adapter that keeps a count of its own isn't made again of its source
alone: its count is state a `map`'s function isn't. A clone of one is
ADR 0542's, an explicit adapter that keeps its count where its clone
reads it (that ADR's amendment). Only a `Map` and a `Filter`, of a source
and a function alone, are made again.

## Amended (#181)

A `Chain` and a `Zip` are cloned apart too: adapters made of two parts
alone, their two sources, as a `Map` and a `Filter` are of their source
and function (`Recognition::cloned_parts`, which says which it is,
`Parted`). `makeChain(a, b)` is `$lazyChain(a, b)`, carrying its clone,
`makeChain` of a clone of each, where it is.

The other side of a `chain(..)` or a `zip(..)` is an `IntoIterator`.
Given as an iterator, it is as the receiver is: its JS value says where
it is. Given as a `Vec`, an array or a slice, it's made the array's
iterator before the maker (`$iter`), and cloned as the iterator the
adapter keeps, so a clone after stepping goes on from there. Another,
whose JS value can't say where it is, `once(x)`'s array, is refused.

And `cycle()` of an adapter cloned apart is `$cycleOf(it, $clone, $next)`:
`cycle` requires `Self: Clone`, so the crate's clones already hold it,
and each round steps a fresh clone of the original, its closures run
again, as std's `Cycle` does.

Held by `test/corpus/cloned_chains_and_cycles.rs`, against native Rust, and
by the mutations `chain-not-cloned-apart`, `zip-not-cloned-apart`,
`zip-side-array-uncloned` and `cycle-of-cloned-apart-refused`.


## Amended (#384): a `copied()` of the crate's own, or of what generic code is given

"A `copied()` is the iterator it copies" held of std's iterators, whose JS
value is their JS iterator, and not of the crate's own, which is stepped
as `$iterator(value, next)`, nor of what generic code is given, which
`$given` makes a JS iterator of where its dictionary is the crate's. A
clone of either was a clone of that JS iterator as the value it steps:
`{ ...copied }`, which threw, or `IClone.clone(copied)`, an empty clone,
`(5, 0)` where Rust gives `(5, 3)`.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the clone is where
the iterator is, apart from it, as Rust's `Clone` of a `Copied` is.

- **Of the crate's own, a clone is a JS iterator of a clone of its
  value**, stepped as it is: `copied.with({ ...copied.of })`, `$iterator`'s
  `with`.
- **Of what generic code is given, `$givenClone(copied, IClone.clone)`**:
  where `$given` made a JS iterator of the value, which it marks `given`,
  one of the value's clone, marked too, so a clone of the clone is one;
  else the value's clone, as before.
- **Refused**: one closed to drop what it steps, the crate's own with a
  destructor or one generic code is given its drop of, `$closing(it,
  drop)`; and a copy of items something changes in place, `source.map((item)
  => ({ ...item }))`, a JS iterator of JS's (`is_its_source`). Each threw
  before; each is in [the limitations](../guide/limitations.md).

Held by `test/corpus/copied_clone_of_its_own.rs`, against native Rust, and
by the mutations `copied-own-clone-spread`, `copied-given-clone-by-dictionary`,
`given-clone-of-the-wrapper`, `given-clone-unmarked`,
`copied-dropped-clone-allowed` and `copied-copying-clone-allowed`.

## Amendment: a chain's other side made an iterator (#1002)

A chain cloned apart whose other side is given as what makes an iterator,
`a.iter().chain(&b)` of a `Vec`, was refused: a clone of its parts would
make `b`'s iterator again from its start. Now its iterator, the chain's
second part, is taken where the chain is made (`as_js_iterator`), so each
part is an iterator, and a clone clones how far it's stepped. Beside a
neighbour that clones a chain of that type, one the crate never clones,
`chain(&b).sum()`, is made so too (the neighbour shuffle, #1002). Corpus
case `chain_of_a_slice_cloned_apart`, mutation
`chain-other-side-not-made-an-iterator`. Generic code's chain that's lent
or of a type parameter not bound by `Clone`, beside a neighbour that clones
one of its type, is still refused: whether the made value can reach a
clone at all is a flow question, #955's.

## Amendment: a chain cloned apart stepped from its back (#1002)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change; a
step from the back must take the chain's last item, as Rust's does.

Once the crate clones a chain type, every chain of that type is made by
its maker, `makeChain(a, b)`, whose form no later step can read, so
`as_double_ended` couldn't make one knowing its back where it's stepped
so: `v.iter().chain(v.iter()).map(f).next_back()` beside a case cloning a
`Chain<slice::Iter<i32>, slice::Iter<i32>>` was refused, "a `Chain<..>` as
a `DoubleEndedIterator`, which rust-js can't step from its back" (the
nightly's neighbour shuffle, seed 12459000).

**A chain cloned apart is marked by the back-stepping analysis as made
knowing its back by its maker where it's stepped so: asked beneath an
adapter, a map's source, or lent to its own `next_back()` (`cloned_chain`
in `stepped_locals`). Its maker then makes `$lazyChain` of its sides made
knowing theirs, as a chain not cloned apart is, which what steps it reads
as knowing its back by that mark; a clone makes the same of its sides'
clones.**

```js
const makeChain = (a, b) =>
  $cloning(
    $lazyChain($iterator(a, $rangeNext, false, $rangeNextBack), $iterator(b, $rangeNext, false, $rangeNextBack)),
    () => makeChain({ start: a.start, end: a.end }, { start: b.start, end: b.end }),
  );
const found = $findBack(makeChain({ start: 0, end: 3 }, { start: 7, end: 9 }), (x) => x % 2 === 0);
```

Whether an adapter is cloned apart is one answer, `iterators::cloned_apart`
of the types the crate clones, which the lowering's `iterator_cloned_apart`
and the analysis both ask. No corpus case's JS changed.

Held by `test/corpus/cloned_chain_stepped_from_its_back.rs`, against native
Rust in both builds: chains of a slice's items, of ranges and of the
crate's own iterator, cloned, and another of each stepped from its back
beneath a map, a filter and as it is; and by the neighbour pair taken off
`test/neighbours-known.txt`. Mutations `cloned-chain-lent-unmarked`,
`cloned-chain-asked-unmarked` (`lower/analysis/back_stepped.ts`) and
`cloned-chain-sides-backless` (`lower/mir/iter.ts`).

## Amended: a generic adapter is a cloned one only where its bounds let it be (#955, #1002)

Generic code's adapter of its type parameters, `Map<I, F>` or `Chain<I, I>`,
was taken for any type the crate clones of that adapter: `instance_of` takes
a type parameter to stand for any type. So a generic function's
`items.map(f).rfold(..)` was refused, as cloning a `Map<I, F>` where `I`
isn't bound by `Clone`, beside another function that clones a
`slice::Iter`'s `Map`, though its `I: DoubleEndedIterator<Item = u32>` is
never a `slice::Iter<'_, i32>`: the neighbour shuffle's #955 and #1002
pairs, each case compiling alone.

**A cloned type is an instance of generic code's adapter only where its
type parameters, given what the cloned type has in their place, meet that
body's bounds, its where clauses (`copies::instance_within_bounds`): each
bound with the parameters so given that mentions no other parameter is
asked of rustc's trait solver, and one that can't hold says it isn't one.**
A parameter given two types, or given another parameter, is one it may be,
as before. The bounds are those of the body that makes the adapter, the
lowering's typing environment, and the analysis's `cloned_chain`, which
asks for each body with its own, so that the one answer is the same of
each. That's a refusal less, not a form changed: no corpus case's JS
changed. The compiler links `rustc_trait_selection` for it (and so the
WASI build).

What's left is generic code beside generic code: a `Chain<I, I>` of one
generic function stepped from its back is still taken for another's cloned
`Chain<J, J>`, as the cloned set doesn't say whose parameters its types
have, nor which values reach which clone (the guide's limitations; since
told apart by their instances where their callers are seen, below).

Held by `test/corpus/generic_map_beside_a_cloned_map.rs`, a generic `map`
and `chain` stepped from their back beside cloned ones of a slice's items,
and a generic `map` its caller clones; and
`test/corpus/generic_map_cloned_where_its_made.rs`, generic code that
cycles its own `map`, still made cloned apart; against native Rust in both
builds; and by #955's and #1002's pairs, taken off
`test/neighbours-known.txt`. Mutations `generic-instance-bounds-unasked`
and `generic-instance-unknown-bound-unmet` (`lower/copies.ts`).

## Amended: generic code's adapter is each instance its callers give it (#1119)

What #955 left: `fn last<I: DoubleEndedIterator<Item = u32>>(a: I, b: I)`'s
`a.chain(b).map(f).next_back()` beside `fn cycled<J: Iterator<Item = i32> +
Clone>(a: J, b: J)`'s `a.chain(b).cycle()` was refused, and with `Item =
i32` in both, whose bounds agree, it still was: rustc names both chains
`Chain<I, I>`, and the cloned set kept `cycled`'s as it's named, any type
its parameters may be.

**A type of generic code's own parameters is each instance its callers
give it, where every caller is seen, by #1009's instance machinery
(`instances::instances_of`), on both sides: a clone in generic code adds
each instance of what it clones to the cloned set (`cloned_types`), and an
adapter generic code makes is asked as each of its instances
(`Cloned::as_any_instance`, which the facts carry with the callers they're
answered from).** Both askers of the one question ask so: the lowering's
`cloned_apart`, by the made type's bounds, and the body facts' choice of
the adapters made in place (`body_queries::cloned_apart`), by the set; so
`cycled`'s own `a.chain(b)` it cycles is made cloned apart, a `$cloning`
maker, as generic code's own `map()` it cycles is. Where a caller isn't
seen, a `pub` function's, which JS calls, or a library's, which its
consumers do, or past `MOST_INSTANCES`, the type stays as it's named and
is asked as before, any type its parameters may be: the refusal stands.
So does a caller that gives both functions the same instance, `last(0..2,
5..7)` beside `cycled(1..3, 4..5)`, whose `Chain<Range<i32>, Range<i32>>`
is cloned apart, one form of one type, which `last` can't make without a
`Clone` bound.

Rust's behavior is preserved, ADR 0262's third case: no result changes,
a refusal less. No corpus case's JS changed: the cases are new.

Held by `test/corpus/generic_chains_cloned_by_their_instances.rs`, the two
pairs, with `chain_cycled`'s chain made cloned apart; and
`test/corpus/generic_clones_of_unseen_callers.rs`, `pub` generic code's
clones, still of any type: its `cycle()` of an `I` stands for no `map` it
isn't given, and the `filter` it gives behind an `impl Iterator + Clone`
is the one its caller clones, cloned apart; each against native Rust in
both builds; and `test/diagnostics.test.ts`'s `pub` pair, still refused.
Mutations `generic-clone-uninstanced`, `generic-cloned-apart-uninstanced`
and `generic-cloned-apart-unseen-callers-unasked`
(`lower/analysis/clones.ts`), `in-place-cloned-apart-uninstanced`
(`lower/body_queries.ts`); `cloned-param-matches-every-adapter` and
`cloned-closures-not-generic`, whose sites only a caller unseen now
reaches, are held by the second case.

## Amended (2026-11-10, #1241): what a function makes behind an `impl` is its value's

`doubled(&v).filter(..).clone()` of `fn doubled(v) -> impl Iterator + Clone
{ v.iter().map(|x| x * 2) }` threw `f[$cloneKey] is not a function`. The
crate's clone of the filter put the `Map` it held among the types it clones
apart, which each of its makers makes a function of its parts, `$cloning`.
But `doubled` made it std's struct, where it's held (ADR 0871). The filter's
clone then cloned that struct as a value cloned apart.

The user's decision (A) on #1226 holds here too: what a function of the
crate makes behind an `impl` is its value's form. `clones.rs` leaves out
std's struct a crate's function makes behind an `impl`
(`recognition::compiled_behind_an_impl`, the one answer, which
`back_stepped` asks too), so it's cloned by its own compiled `Clone`. A
library's `impl .. + Clone` is its library's to make, cloned apart where it
gives one out, as its contract says (#1032, #1202). Case C
([0262](0262-when-rust-and-js-disagree.md)): a program threw where Rust
runs. `test/corpus/clone_of_an_adapter_over_a_returned_impl.rs` holds a
filter's, a map's and a chain's clone of it, in both builds.

And a library's alike (#1242): it makes what it gives behind an `impl`,
where it's held, std's compiled struct, by the same rule the crate does
(`compiled_behind_an_impl`, for any crate's opaque type). A consumer's clone
of a filter over a library's `impl Iterator + Clone` cloned the library's
struct as one cloned apart, `f[$cloneKey] is not a function`. That struct is
never cloned apart, yet its parts are still cloned in turn: its compiled
`Clone` clones its fields, a closure among them, which its library's
contract says how (#1202).
