# 0145. A generic function is given the size and name of a type parameter it asks for

Status: Accepted. Extends [0049](0049-traits-and-generics.md), [0090](0090-wasm32-front-end.md)
and [0098](0098-destructors.md). Extended by [1115](1115-what-generic-code-needs-of-its-type-parameters-has-one-catalogue.md): its facts are the generic facts' catalogue's given ones. Amended:
`mem::needs_drop` is a fact too (below); and a trait impl's method asks
its impl what it asks of the impl's type parameters: the impl's
dictionary is made of them, `wrapFootprint(TSize)`, one for each set,
keyed in a `Map`, and gives its methods each, which a direct call of one
is given too. A trait's default method and an impl's method of its own
type parameters ask none still (`test/corpus/impl_type_facts.rs`,
`impl_const_size.rs`). And a library's function or impl its consumers can
reach asks too: its manifest says what, `type_facts`, its consumers' calls
give it, and a consumer's generic function that passes its own type
parameter on asks it too, arrayvec's `extend_from_slice` of its public
`try_from` (`test/crates.test.ts`). A trait's method called of a type
naming the caller's type parameter is its impl's, and asks what that
asks: arrayvec's `from_iter` calling `array.extend(iter)` of an
`ArrayVec<T, CAP>` asks `T`'s size, and passes it on; so does one that
gives an impl's dictionary for a callee's bound, a `C: Push<T>` of a
`Stack<T>`, or calls a std method that only calls its bound's,
`collect()` that's `FromIterator::from_iter` (ADR 0160), arrayvec's
`Clone` (`test/corpus/impl_type_facts_through_trait.rs`). Which impl
gives the dictionary is `traits::evidence_impl`'s alone. And a fact of an
associated type of the crate's trait, `size_of::<A::Item>()` of smallvec's
`A: Array`, asked directly or of a callee given it, is its trait's
dictionary's, `AArray.ItemSize`: each impl's dictionary carries it of its
type for it, `T`'s of `type Item = T`, which its factory is given
(`test/corpus/associated_type_facts.rs`). A type's layout fact is
`type_fact_value`'s alone, `size_of_val` of a value's bytes apart.
Extended by [0466](0466-a-generic-maps-class-is-its-callers.md): a map's
class, of a trait's generic method's own type parameters and of a std
associated type of one.

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`size_of::<u32>()` is `4`, rustc's answer for the wasm32 target rust-js
checks programs for (ADR 0090), and `type_name::<u32>()` is `"u32"`. Of a
type parameter, they were errors: a generic function is one JS function
for every type, so its `T` has no one answer.

```rust
fn describe<T>(_: &T) -> String {
    format!("{} {}", type_name::<T>(), size_of::<T>())
}
```

Each caller knows its type, as each caller of a function with a destructor
to run knows its drop (ADR 0098).

## Decision

**A generic function that asks `size_of`, `align_of` or `type_name` of a
type parameter takes the answer as a parameter, after its dictionaries,
and each caller gives it for its type:** (Amended by ADR 0331: and its
`TypeId`, `TId`, unless a `T: Any` bound gives its dictionary.)

```js
function describe(_, TSize, TName) {
  return `${TName} ${TSize}`;
}

describe(1n, 8, "u64");
```

- **Asked directly, in a closure, or through a function it calls:** a
  generic function of the crate's that passes its own `U` on as a callee's
  `T` asks what the callee asks, `relay(u, UClone, USize, UName)` giving
  `describe(u, USize, UName)`. The analysis runs backwards from each ask,
  as `drop_params` runs forwards from each value with a destructor.
- **A fact is one value:** `size_of` and `size_of_val` of a sized type ask
  its size, `align_of` its alignment, `type_name` and `type_name_of_val` its
  name, each once, in that order, after the dictionaries and before any
  drop function.
- **A caller's concrete type answers it,** as rustc computes it for
  wasm32; a caller's own type parameter passes on what it was given.
- **`size_of::<T>` as a value is a call's arrow**, `let size =
  size_of::<T>` `const size = () => TSize`, and of a concrete type, `() =>
  2`. (Amended: a value of it was its own case, of a concrete type only.)
- **Still errors:**
  - through a trait's dictionary, a trait method's or an impl's, whose
    callers can't see what it asks, and a `Drop` impl's;
  - of an associated type, `T::Native`, or of a type holding a parameter,
    `type_name::<Cow<'a, T>>()`;
  - `size_of_val` of an unsized `T`, which is the value's size;
  - of a library's function its consumers reach (ADR 0100), which never
    see what it asks.

## Why

- **It's exact:** the value native Rust's monomorphized code has, for the
  wasm32 target, compared with native Rust by the `type_facts` corpus case
  on types whose layout is the same on both.
- **It's the JS a person writes:** a number or a string passed in, and a
  function that asks nothing takes nothing.

## Alternatives

- **One object per type parameter, `{ size, align, name }`,** given to
  every generic function. Simpler to pass on, but every generic function
  would take one, most for nothing.
- **A dictionary entry,** as a bound's. A size has no bound to hang on:
  every `T` is `Sized`.

## Consequences

- 5 rustc tests pass that stopped at a size or a name of a type parameter.

## Amendment: `needs_drop`

`mem::needs_drop::<T>()` (2026-10-15) is a fact of `T` too, `TNeedsDrop`
where it's a type parameter's: rustc's answer, `true` or `false`, which is
what the program sees, as native Rust's. It was refused. The facts are one
list, `TypeFact::ALL`, each named by its `word`, which the analysis that
passes them and the parameters that take them both ask. The corpus's
`needs_drop.rs`, and rustc's `union/union-manuallydrop-rpass.rs`, pass.

## Amendment: a name of an unsized type parameter, and of a type that holds one

`type_name::<T>()` of a `T: ?Sized` asked nothing of its caller, as a size
of one isn't one size; a name is, so it's asked, and given, as of any
(a size or an alignment, of an unsized one, still isn't). And of a type
that holds a type parameter, `type_name::<Cow<'_, T>>()`, it's rustc's name
with the one its caller gives of each parameter in its place:

```js
function names(TClone, TName) {
  return [`alloc::borrow::Cow<'_, ${TName}>`, `alloc::vec::Vec<(${TName}, u8)>`];
}
```

rustc names a parameter `_`, so each is put in as a type no program names,
`[!; 4000000000]` and up, by its index, and its given name put where that
is. **A type's fact of a `dyn` given to generic code is refused:** over a
`T: ?Sized`, it resolves a blanket impl's method where rustc calls the
`dyn`'s own, its value's type's, rustc's `dont-propagate-generic-instance`.
The `type_names_generic` corpus case names types holding a parameter, an
unsized one, one passed on, and a blanket impl's, directly and through a
`dyn`, with native Rust; rustc's `type-name-basic` passes.

## Amendment: an impl's facts reach a caller one way

**What an impl asks of its type parameters reaches a caller through the
dictionary the caller gives for a bound**, a std method's too:
`collect()`'s `B: FromIterator`, `parse()`'s `F: FromStr`. A std method
that only calls its bound's, `collect()` that's `from_iter`, was also
resolved to that impl's method, a second way to the same facts: gone, and
the mutation that held it, which survived. A trait's method called of a
type naming the caller's type parameter is still resolved to its impl's:
a std trait's that's no dictionary's, `From::from`, smallvec's
`SmallVec::from(slice)`, reaches its impl no other way (its corpus case
holds it).

## Amendment (2026-11-02): a library's trait's associated types' facts are its manifest's (#476)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change: a
fact read of a dictionary that doesn't carry it is `undefined`, so what
isn't carried is refused.

Since ADR 0914 (30a9fb024), a consumer that gives smallvec's `SmallVec<A>`
an item with a destructor compiles its methods itself, from smallvec's
MIR, `inline_capacity`'s `size_of::<A::Item>()` among them. The fact is its
`A: Array` dictionary's, `AArray.ItemSize`, which smallvec's
`_T__N_Array(N, TSize, dropT)` carries; but only a fact of the crate's
own trait was a dictionary's (`carried_by`), so the consumer's was refused,
"`size_of` of a type parameter", and smallvec's probe with it. And a
consumer's own impl of a library's trait carried none of what the
library's code asks of its associated types: a dictionary of it would give
`undefined`.

**A library's manifest says what each of its traits' dictionaries carry of
each associated type, `associated`, by key, of `type_fact_params`' own
entry of it; a consumer's dictionaries of its own impls of the trait carry
those too, and its code reads of any dictionary of the trait what the
manifest says, `AArray.ItemSize`.** A fact the manifest doesn't list is
refused as before, as no dictionary of the library's carries it.

```js
// smallvec's consumer, its own `Array` for its `Pairs`, and its compiled
// `inline_capacity`
export const pairsArray = { size: pairsArray_size, ItemSize: 2 };
  if (AArray.ItemSize > 0) {
```

The library contract is **library ABI 13** (`library::VERSION`): its
manifest's shape changed, so a library compiled before must be compiled
again, and `tooling/manifest.js` checks the new field.

- Held by `test/crates/pairs/item_size_of_instance`, against native Rust:
  a consumer giving the library's `Small<A: Array>` an item with a
  destructor and one of no size, compiling its methods itself; by
  `library_trait_facts_of_own_impl`, a consumer's own `Array`s, whose
  dictionaries the library's `Default` reads; by the refused pair
  `uncarried_associated_fact`, a consumer's `size_of` of a library's
  associated type that its dictionaries don't carry; and by
  `test/manifest.test.ts`'s library contract. Mutations
  `library-associated-uncarried`, `library-trait-impls-carry-none`,
  `uncarried-associated-read` (`lower/analysis/type_facts.ts`),
  `associated-facts-unexported` (`lower/library.ts`) and
  `associated-facts-unread` (`library.ts`).

## Amendment (2026-11-04): what std collects an iterator in, in place (#672)

A fact of an iterator, `InPlace`, `IInPlace`: the `[size, align]` of the
items of the buffer std's `collect()` into a `Vec` may collect it in, in
place, or `undefined`, by which a collect a panic stops drops what it
collected or what's left first. Asked of the type parameter whose
iterator the adapters keep, and given by its caller, of the same
`recognition::in_place_source`; of a type that holds one, `Map<J, F>`'s,
`J`'s ([0465](0465-dropping-an-adapter-is-closing-it.md), amended).

## Amendment (2026-11-05): `from_iter`'s iterator is its `T` (#737)

The analysis reads what a call collects of `recognition::collected_items`,
the one answer the lowering's `rest_dropped_first` reads too. Of
`<C as FromIterator<A>>::from_iter::<T>`, it read the iterator off `A`, the
items, not `T`, what it's given. So `Vec::from_iter(iter)` of a type
parameter's iterator asked its caller for nothing, and the lowering
refused it (`the buffer std collects an iterator in place in`, or `size_of`
of a type parameter, as itertools' `sorted_by` refused). It reads `T` now,
so the caller gives `IInPlace`, `TSize` and `TAlign` as it does of
`collect()`.

## Amendment (2026-11-05): the layout of a type built of the slots (#737)

A collect into a `Vec` in generic code whose items are a type built of
its type parameters, `Held<<H::Item as IntoIterator>::IntoIter>`, as
itertools' `multi_cartesian_product` collects `MultiProductIter`s, compares
that type's layout with its source's (#672). No bound names the type, so
no caller could derive it as it derives `drop_items`, and the lowering
refused it, "`size_of` of a type parameter".

**Each function has built slots, after its type parameters' and their
associated types': each type built of them whose size or alignment it
asks, which rustc can't say there.** The type-facts analysis finds them
where a collect asks its items' layout, or where a callee's fact of a
slot, or of a built slot, is of such a type in the caller's terms,
`Held<IntoIter<T>>` of a `relayed<T>`, and numbers each function's in the
order of the types' names (`traits::drop_slot`, `built_slot`). A caller
gives the layout as it gives any fact, `HeldSize`, `HeldAlign`, of the
type made of what it gives; the callee reads it as it reads an
associated type's (`type_fact_value`).

A built slot's layout is read only by a collect that may be in place, so
it has **gates**: the slots whose `IInPlace` its collect reads. A caller
giving each gate a source std never collects in place, `undefined`, gives
the layout none: `undefined`, or nothing at the end of the call
(`gives_slot`, the user's decision). A generic caller passes the gates on
in its own terms, and asks nothing where each is never in place.

A library's manifest says each built slot's type as a template of the
function's slots, `TypeTemplate`: a slot by its index, a struct or an
enum by rustc's `DefPathHash`, a tuple, a reference, an array or a
primitive, with its gates. A shape the template can't say, a const
argument, is refused where the library is compiled. This is **library ABI
16**.

Two more of the same family: a caller giving an impl's dictionary for a
type of its slots that isn't one of its type parameters,
`impl<T: Iterator> Itertools for T` of an `<I as IntoIterator>::IntoIter`,
asks what that impl asks, as of a call (#794's itertools part). And the
analysis's answer to whether a collect's items drop is the lowering's,
`drops::drops_given`: it asked `drop_runs`, which says a `G<J::Item>`'s
associated type may drop, unsupported, where the lowering's drops say
`G`'s own `Drop` runs; so a library's `pub fn` collecting those asked
nothing, and its lowering refused it.

- Held by `test/corpus/in_place_collect_of_a_built_type.rs` and
  `in_place_collect_of_a_built_type_relayed.rs`, against native Rust in
  both builds: in place, not in place and a range's map; relayed through a
  generic caller; a trait's default copied into a blanket impl; a tuple
  built of a slot; the blanket impl given `I::IntoIter`. And by
  `test/crates/pairs/built_layout_of_a_library`, a library's `held` and
  `tagged` its consumer calls through the manifest's templates. Mutations
  `built-layout-*`, `built-slot-misplaced`, `built-tuple-unnamed`,
  `impl-of-a-slot-type-unasked`, `copied-default-own-clauses-unknown`,
  `built-template-*` and
  `items-drop-asked-*`.

## Amendment (2026-11-09): a trait's dictionary carries its std supertrait's item facts (#990)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change: a
fact read of a dictionary that doesn't carry it is `undefined`, so what
isn't carried is still refused.

itertools' `Itertools: Iterator` asks `size_of::<Self::Item>()` in its own
defaults' bodies, `collect_vec`'s collect among them. A library's default
is compiled over `Self` for each consumer impl that doesn't write it
([0185](0185-library-trait-defaults.md), amended), and no caller gives it a
slot, so where no bound names `Self::Item` nothing could give the fact,
and the library was refused, "`size_of` of `<Self as
std::iter::Iterator>::Item`, an associated type whose facts its caller's
dictionary doesn't carry". A plain generic function's `size_of::<I::Item>()`
was refused too: only a map's keys asked the slot.

**A carried fact is keyed by the carrying trait, the associated type and
the fact. A trait whose default asks a fact of an associated type of a std
supertrait of its `Self`, `<Self as Iterator>::Item`, has its dictionaries
carry it, `IteratorItemSize`, which its default compiled over `Self` reads
of the dictionary it's given.** Each of the crate's impls that copies the
default asks its own type for it of its slots, as a collect's are asked
(`collect_asks`), so a copy reads it as it reads any of its impl's facts.
And a function's `size_of`, `align_of` or name of a std associated type of
a type parameter asks the slot, its caller's to give, as a map's keys'
class does (ADR 0466).

```js
// a consumer's impls of a library's `Tools: Iterator`, and the library's
// `item_layout` it compiles over `Self`
export const countdownTools = {
  itemLayout: (arg0) => tools_itemLayout(arg0, countdownTools),
  IteratorItemSize: 4,
  IteratorItemAlign: 2,
};
function tools_itemLayout(self, SelfTools) {
  return [SelfTools.IteratorItemSize, SelfTools.IteratorItemAlign];
}
```

A library's manifest says what they carry, an `associated` entry with a
`carrier`, the trait's key; a consumer's dictionaries of its own impls of
that trait carry those, of the impl's type: a type it knows, or its own
slot, `IItemSize` of an `impl<I: Iterator> Tools for Wrap<I>`. The crate's
own trait's defaults are copied into its impls, so a dictionary of the
crate's own trait carries none. This is **library ABI 24**.

The other three, the map keys of `<Self as Iterator>::Item`: a default's
map's class is its method's slot, which its caller gives (ADR 0466), as a
library's copy of it takes it, `counts(self, SelfItemHash,
SelfItemPartialEq, SelfItemMap)`. A consumer compiles the library's default
from its MIR, whose analysis asked no map's class (only a body read from its
THIR did), so it was refused, and a consumer's call of the library's copy
through its dictionary gave none. **What a map of a type parameter's keys a
std function makes asks is one answer, `map_made`, of a body read from its
THIR or its MIR alike**, so the consumer's calls give the class the
library's copies take. And a consumer's impl's entry for the library's
default gives it of the slot's type for that impl, `<I as Iterator>::Item`
of a `Wrap<I>`'s, as the default asks it, named as its bounds' dictionaries
are, `WrapItemMap`.

- Held by `test/crates/pairs/library_default_item_layouts`, against native
  Rust: a consumer's impls of a known item and of a type parameter's, read
  through a generic caller, and the library's own impl. And by
  `test/corpus/size_of_an_iterators_item.rs`, in both builds: a generic
  function's `size_of` of `I::Item`, and a trait's default copied into a
  blanket impl and a known one. And by
  `test/crates/pairs/library_default_map_keys`: maps of `Self::Item` keys
  and of a method's own type parameter's, through a consumer's impls of a
  known item and a type parameter's, a generic caller and the library's own
  impl. Mutations `super-item-*` (`lower/analysis/type_facts.ts`,
  `lower/traits.ts`, `lower/library.ts`), `map-made-unasked-of-mir` and
  `own-fact-given-in-trait-terms`.

## Amended: a slice's `size_of_val` (2026-11-09, #1101)

`size_of_val` of a slice, a value of no one size, was refused, and so was
rand_core 0.10, whose `read_words` asserts `size_of_val(src)` of a `&[u8]`
(found by the small Crater, #1087). A slice's size is its length times its
item's, as the slice's `PtrMetadata` is its `length`: `src.length`, of a
wider item `s.length * 8`, its item's size rustc's for wasm32 as any
`size_of` is; `align_of_val` of one is its item's alignment, what's
measured still run where it does something, as any `align_of_val` does. Case N: rustc's answer, kept. A
`str`'s and a `dyn`'s are still refused. `test/corpus/size_of_val_of_a_slice.rs`,
against native Rust in both builds; mutations `slice-size-of-val-item-unscaled`
and `slice-align-of-val-its-size`.

## Amendment (2026-11-11): how std collects a `Vec`, `IVecCollect` (#1239)

std's `SpecFromIter` collects into a `Vec` one of three ways, by the
iterator's type: in place, in the buffer a `vec::IntoIter` holds, asking
nothing of its hint; by its fold, a `TrustedLen`'s, room for its hint's
upper bound first, or a "capacity overflow" panic where it has none,
stepping nothing; or by `next`, its first item, then room for its hint as
the `Vec` grows. Generic code's `collect()` or `Vec::from_iter` of a type
parameter's iterator knew none of them: it asked no hint, stepped an
endless `TrustedLen` before the panic and folded no `TrustedLen` by its
fold. Which way is a fact of the iterator, `VecCollect`, `IVecCollect`:
`"inPlace"`, `"fold"` or `"next"`, a string, which the concrete caller
computes of `recognition`'s answers, the in-place rule
(`collects_in_place`) and `is_trusted_len`, and a generic caller passes on
as it's given it. Of an iterator generic code builds of its own type
parameters, `Map<I, F>`, it's made of their facts, which `held_facts`
names: `$vecCollectWay($collectsInPlace(IInPlace, size, align),
ITrustedLen)`, each part only where it's a caller's. Asked by the facts' owner of a collect into a `Vec`,
`recognition::collected_items`'s, whose iterator is so given
(`vec_collected`), in place of `IInPlace`, which it answers: one question,
one fact, one path. A `B: FromIterator`'s entry, which knows its items where
a generic caller may not, itertools' `MapSpecialCase::collect` of an
`R::Out`, is given `IInPlace` and `ITrustedLen` of its iterator, as std's
`from_iter` declares them (`std_method_facts`), and makes its way of them,
`vec_collect_way_of`; and it makes the facts its impl's method asks of
its own type parameter under the trait method's bounds for the impl, `T:
IntoIterator<Item = i32>`, so a crate's `from_iter` that collects a `Vec`
of what it's given knows its items. A JS caller
leaves it out, `undefined`, `Form::LeftOut`'s, as it leaves out a drop:
what it gives is no Rust iterator, which `undefined` collects by `next`,
as JS steps it; so an exported generic collect stays declared (ADR 1077).
The rule is one on each side, the compiler's `vec_collect_way` of a type
known and the runtime's `$vecCollectWay` of what a generic caller's
caller gives, and `test/vec-collect-ways.txt` holds both to one table,
`cargo test vec_collect_ways` and test/runtime-package.test.ts. One runtime
call collects by it, `$collectVec(it, IItemSize, IVecCollect, drop)`, and
of items that drop, drops them as a panic unwinds each way as std does: in
place by its `try_fold`, what's left of the source after what it
collected; by fold, what's left first, as `extend_trusted`'s fold unwinds;
by `next`, what's left first (#423). `ITrustedLen` keeps its own
use, what std's zip reads ([0975](0975-std-s-zip-is-compiled-its-impl-picked-by-its-sides-facts.md)). Held by
`test/corpus/vec_collect_as_its_caller_knows.rs`, against native Rust, and
the metamorphic net's `generic-instances` rows of
`zip_of_a_boxed_map.rs`, `collect_endless_once_stepped.rs` and
`fold_drops_moved_function_first.rs`, now off its list, and the whole
corpus's `generic` and `generic-instances` rewrites. Mutations
`vec-collect-*` and `collect-vec-*`.

## Amendment (2026-11-11): what a default's call asks, each copy asks (#1313)

A trait's default copied into each of the crate's impls that doesn't write
its own, `impl<T: Iterator> Tools for T` (ADR 0049), asked of the impl what
its collects ask of the trait's `Self`, in the impl's terms, `T`. What a
function it calls asks of what the default gives it was asked of the
default alone: itertools' `multi_cartesian_product(self)`, whose callee
collects into a `Vec` std may make in place, asks `IInPlace` of `Self`, and
the copy in the blanket impl, which its caller calls, was given none, so
it was refused, "the buffer std collects an iterator in place in". Both now
take one translation, `copies_of_default`: a default's `args`, in each
copying impl's terms and environment as the lowering has them
(`default_args`, `default_typing_env`), which `collect_asks` and the calls'
fixpoint read. Of a call, the slots' facts alone, `T`'s of `Self`: the
layout or map class of a type built of them, asked of a copy, its caller
asks of what it gives, and a default that calls the blanket impl's copy at
`Enumerate<Self>`, itertools' `minmax_by_key`, asks it again of
`Enumerate<T>`, and so on, which no finite set of facts settles (#1346,
where itertools is refused now, past #1313's site).
Held by `test/corpus/default_gives_its_callee_what_self_collects_in_place.rs`
against native Rust, a panic part-way among it; mutation
`default-call-facts-unasked-of-copies`.

## Amendment (2026-11-12): a library's traits carry what its impls can give of their associated types' layouts (#1420)

A library's trait's dictionaries carried only the facts its own code asked
of an associated type (#476). So a consumer's generic `size_of::<Self::Item>()`
was refused, "`size_of` of a type parameter": rand 0.7's `ReseedingCore` over
rand_core's `BlockRngCore`. The user's #1341 choice was that every trait's
dictionaries carry what a consumer may ask, so a library's reachable trait
now carries the `Size` and `Align` of each sized, non-generic associated
type (`carried_by_every_impl`); a method's `impl Trait` is one whose
layout no impl gives, its manifest lists none. Its manifest
lists them, library ABI 29. A consumer's own impls of the trait carry them
too, as they carry any listed fact.

Not every trait can carry them. The library computes each impl's type's
layout, and an impl can't give one where its type is a type built of its
parameters that a contract template can't say, or another trait's associated
type: ppv-lite86's `x2<<W as AndNot>::Output, G>`, a `dyn` beside a
parameter, typenum's `<<U as Len>::Output as Add<B1>>::Output`. Carrying the
facts would refuse a library that compiles now. A trait carries them only
where each of the library's impls can give them (`layouts_given`), and
otherwise carries none, as before (the coordinator's choice C). A
`Projection` template node, which would carry ppv-lite86's, waits until a
crate needs it.

On the way, a type built of type parameters whose layout rustc knows
whatever they are, `Box<dyn Fn(&T) -> usize>`, a sized `T`'s `&T`,
`PhantomData<T>`, has rustc's size in generic code (`layout_bytes`). It was
refused as a type parameter's.

Held by the crate pair `size_of_a_library_traits_item`, against native Rust:
dep's `Words` and the consumer's own impls asked through a generic wrapper,
and dep's traits that carry none, `?Sized`, generic, `impl Trait`, an
untemplated impl type, another trait's type and a std projection. Also held
by `test/corpus/size_of_what_rustc_knows_of_any_parameter.rs` in both builds.
Mutations `*-assoc-layouts-*`, `unknown-layout-assoc-carried` and
`rustc-known-generic-layouts-refused`; the test "a library's traits carry
the layouts of the associated types their impls can give" reads its manifest.
