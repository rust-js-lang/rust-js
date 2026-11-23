# 0061. `impl Iterator` is the type it hides; a generic iterator is any JS iterable

Status: Accepted, amended by [1113](1113-a-bodys-impl-trait-is-revealed-where-its-mir-is-taken.md):
what an `impl Trait` hides is read once, where a body's MIR is taken. Amended by [0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md):
an `IntoIterator` whose iterator is held as it is is given as it is.
Extends [0036](0036-iterators-and-sorting.md) and [0055](0055-iterator.md).
Amended by [0421](0421-std-associated-type-drops-are-given.md): the JS
iterator made of one of the crate's, `$iterator(value, next)`, has `of`,
`value`, which its drop drops; and a trait impl's method is given one, as
a function is.
Amended by [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md):
a parameter an impl's type holds, or a signature has in anything but
itself, is given as it is, and stepped by its `Iterator` dictionary.
Amended by [0429](0429-a-double-ended-iterator-knows-its-back.md): one
bound by `DoubleEndedIterator` is a JS iterator that knows its back.

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An iterator has two JS forms:
- a std one over a slice, a `Vec` or a range is an array (ADR 0036);
- one of the crate's own is a JS iterator, made by `$iterator` (ADR 0055).

Neither covered the two ways Rust code hands iterators around:

```rust
fn evens(n: u32) -> impl Iterator<Item = u32> { (0..n).filter(|x| x % 2 == 0) }
fn total<I: Iterator<Item = u32>>(items: I) -> u32 { items.sum() }
```

An `impl Iterator` was an error: an opaque type is none of the types
rust-js knows. A generic `I` could be either form, depending on the caller.

## Decision

- **An `impl Trait` type is the type it hides.** Once type checking is
  done, rustc knows what `evens` returns, a `Filter` of a range, and
  rust-js reveals it wherever it decides by type: an array here, `$range(0,
  n).filter(..)`. An `impl Iterator` hiding one of the crate's own
  iterators is that struct, as it is in Rust.
- **Generic code takes an iterator with `Iterator.from(items)`.** It
  accepts an array and a JS iterator alike, and gives JS's lazy helpers
  (ADR 0055): `Iterator.from(items).drop(1).take(k).toArray()`. A `for`
  over an `I: IntoIterator` is `for (const x of items)`, which also takes
  either.
- **One of the crate's own iterators, given where a generic one goes, is
  made a JS iterator first:** `total($iterator({ n }, countdownIterator_next))`.
  An array needs nothing.

## Why

- **It's the same JS either way.** A caller's array or iterator reaches
  generic code as what it is, with no dictionary: `Iterator.from` does the
  one thing that differs.
- **Revealing the type is exact.** It's the type Rust itself uses when it
  generates code.

## Consequences

- An `impl Iterator` is the type it hides wherever its JS is asked for, a
  `&mut` to one too, as `next()`, `any` and `find` take it: `tens(v).next()`
  is `tens(v)[0]`. (Amended: a `&mut` to one a function returned was a box,
  so `next()` of it was `None`, and `any` of it a `TypeError`.)
- An associated type bound by `Iterator` where its trait declares it,
  `type Iter: Iterator`, is a generic iterator as a type parameter bound
  so is: a `&mut` to one, `advance(&mut items)`, is the JS iterator it
  steps. (Amended: only where-clauses were read, so a `&mut` to one was a
  box, and what stepped it was given the box, a `TypeError`;
  `generic_associated_types` runs it against native Rust.) And an impl's
  value for one, a range or an iterator of the crate's, `type Iter =
  Range<u32>`, is made a JS iterator where its trait's dictionary gives
  it, as one given where a generic iterator goes is; a call of the impl's
  own method still gives the range. (Amended: the range itself, which
  generic code's `for .. of` threw on.)
- `next()` on a generic iterator is still an error. Generic code uses the
  adapters, `for`, and what consumes one.
- A range handed over as a value (`total(0..3)`) is still an error, as a
  range in a variable is (ADR 0036).

## Amendment: a type parameter's associated type too

`i.into_iter()` of `H::Item: IntoIterator`, itertools'
`multi_cartesian_product`'s, was refused: only a type parameter was
taken for what its caller gives. An associated type only a caller knows
is the same, an array or a JS iterator, stepped by `Iterator.from`:
`is_given_iterable` answers it, for `into_iter`: a type parameter, or an
associated type that isn't an iterator. One that is, `I::IntoIter`, and a
generic iterator's value of an associated type, stay as they are: a JS
iterator already, as `into_iter` or a dictionary gave it.
`test/corpus/associated_into_iter.rs` steps them against native Rust.

## Amended by

[ADR 0715](0715-what-an-into-iterator-bound-is-given.md): what a type parameter bounded by `IntoIterator` is given is `iterable_items` of its argument, a `&mut` collection's handles among them, and a `&mut` range is stepped where it is.

## Amendment: a function given as a value converts what its call does (#281)

A call of the crate's generic function converts each argument as
`given_iterators` says, for example an `Option` that an `IntoIterator`
bound takes, `x == null ? [] : [x]`. The same function given as a value,
`map(total)`, was its bare name, so each item reached it unconverted, and
`for (const x of 1)` threw. Following ADR 0262's "results change" case,
Rust's answer is kept: `fn_item_value` asks `given_iterators` whether the
call converts any argument, `converts_arguments`. If it does, the value
is the arrow of its call, `(x) => total(x == null ? [] : [x])`. One whose
call gives each argument as it is stays its name, `map(total)` of `Vec`s.
`fn_item_converting_argument` runs both against native Rust, and the
mutation `fn-item-conversion-unasked` is caught.

## Amendment: only where an iterator goes (#991)

The crate's own iterator given for a type parameter was made a JS
iterator whatever the parameter's bounds, `$iterator(value, next)`, so a
`T: Debug`'s dictionary read the wrapper as the struct, `Gen(undefined)`,
and a `T: Clone`'s cloned it. `given_as_iterator` asks the callee's bound
first, as its doc said: one bound by `Iterator` or `IntoIterator` is given
the JS iterator, one bound by neither the value itself, which its
dictionaries and its drop read (`iterated_slot` asks the same). Corpus
case `iterator_given_where_no_iterator_goes`.

## Amendment: what a compiled std body is given (2026-11-16, #1962)

A std body compiled into `@rust-js/std` (ADR 0561) holds what it's given
as it is and steps it through its dictionary, whose `next` is `$next`, a
JS iterator's `next()`. Given a type parameter's value stepped as JS's,
one given no dictionary of its own, `items.peekable()`, `items.fuse()`, a
flipped `items.enumerate()` or `items.chain(..)` (ADR 1136), it stepped a
JS caller's array by `.next()` and threw `it.next is not a function`,
where this ADR says an array may be given. Such a value given to a
compiled body as an `Iterator`-bound argument is now `Iterator.from(items)`,
as the hand-written forms step it (`given_iterators`). One given its
dictionary is what that dictionary steps, as it is, and an `IntoIterator`
argument is its dictionary's to make an iterator of, as it was.

Results changed, so this follows ADR 0262's third case. 46 corpus
snapshots gain the `Iterator.from` (10,992 lines to 11,025), each as
native prints. Test: `lowering-generic-iterators.test.ts`' "a std body
compiled given a generic iterator takes an array too"; mutation
`compiled-given-a-param-as-it-is`.

## Amendment: an `impl Iterator` within what a trait method gives (2026-11-17, #2005)

An impl's dictionary entry makes what its method gives the JS iterator
generic code steps where the trait declares an associated type or an
`impl Iterator` bound by `Iterator` (#1292). It looked at the whole result
alone, so one within it, `-> (impl Iterator<Item = u32>, u8)`'s field or
`Option<impl Iterator>`'s payload, reached generic code as the impl made
it: a range's `{ start, end }`, where `it.sum()` threw `it.reduce is not a
function`. `made_for_generic_code` (traits/impls.rs), the one answer, walks
a tuple's fields, nested ones too, and an `Option`'s payload, each made as
its own would be, read of the value the entry holds first: `const given =
aSrc_tup(arg0); return [$iterator(given[0], $rangeNext), given[1]];`. A
result that holds both a `&mut` generic code's box adapts and an
`impl Iterator` was refused, naming #2010, until the amendment below.
Corpus case
`impl_iterator_within_a_trait_methods_result` holds a range, the crate's
own iterator, one stepped from its back, a nested tuple's, an `Option`'s
`Some` and `None`, and items and a field with a destructor against native
in each build; mutations `impl-iterator-within-tuple-unmade` and
`impl-iterator-within-option-unmade` are caught.

## Amendment: a `&mut` beside an `impl Iterator` in what a trait method gives (2026-11-17, #2010)

An impl's dictionary entry adapted what its method gives in two passes
over the one value: each `&mut` generic code holds as its box (`Within`,
#686) and each `impl Iterator` made the JS iterator generic code steps
(`made_for_generic_code`, #2005). A result needing both, `fn pair(&mut
self) -> (&mut Self::Item, impl Iterator<Item = u32>)`, was refused.
Both walk the same tuples and `Option`s, so `made_for_generic_code` now
walks `Within` beside them: at each part that isn't a tuple or an
`Option`, an iterator is made as before, and anything else is
`Within::given_back` of it. The cell is made in the method's
environment, where the iterators are made. That environment holds the
impl's bounds, and a generic impl's `&mut T` was the same in either
environment. One array comes out: `const given
= heldSrc_maybe(arg0.value, arg1); return given != null ?
[$cellOf(given[0]), $iterator(given[1], $rangeInclusiveNext)] :
undefined;`. A result that isn't a tuple or an `Option` keeps its `&mut`
adaptation apart, as it was.

Corpus case `mut_ref_beside_impl_iterator_in_a_trait_methods_result`
holds a `&mut` before an iterator and after one stepped from its back,
and an `Option` of both, of a generic impl's `&mut T` too. It covers an
object's and a number's `&mut`, an
item with a destructor written through each `&mut`, and an `Option`'s
`None`. It matches native in each build and through every metamorphic
rewrite. The diagnostics row and the guide's limitation are gone.

## Amendment: an iterator JS holds as no object is refused where generic code is given it (2026-11-21, #2254, #2255; superseded below)

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't be
preserved yet, so it's a compile error. Generic code holds an iterator type
parameter, and a `&mut` of one, as the value itself, which assumes the
iterator is a JS object. One of the crate's that is no object breaks that:
a fieldless enum's string, or a transparent struct's number. Its own
`&mut self` steps take its box or place.
- Given by value, generic code steps the bare value. `Light::Green.take(3)`
  threw "Cannot create property 'value' on string 'Green'".
- Given as a `&mut`, generic code gets the caller's place handle and
  takes it for the value. It clones the handle and writes it back over
  itself, `*it = old`, so a step it meant to undo stays. That is a silent
  wrong answer (#2254).

**`generic_facts::iterator_held_unboxed` says which crossings these are.
The call's arguments (`mir/assign/boxed_args.rs`) refuse them, naming
#2255.** A crossing counts when the iterator is the crate's own, isn't an
object, and has more than one value (not `one_valued`), and:
- it's given by value to a type parameter bound by `Iterator` or
  `IntoIterator`, or
- it's given as a `&mut` (`&mut I`, or an `I` given `&mut x`) to one also
  bound by a trait beyond the iterator traits, `Sized` and auto traits. That
  bound may clone, compare, format or write it whole.

These still compile:
- a `&mut` of one bound by the iterator traits alone, `l.by_ref().take(2)`,
  which only steps the place, as the iterator's own steps do;
- the iterator's own `next()`, and a `for` loop over it;
- a unit struct, which has one value, so no write or clone of it can be
  told apart. Its one failure is #2253's crash.

The fix is #2255: generic code is given such an iterator boxed, with its
dictionaries adapted; this refusal goes when it lands. Test
`test/diagnostics.test.ts`'s "a fieldless enum's iterator given to generic
code as an iterator is refused" and the guide's limitation hold the
refusal. Mutations `unboxed-iterator-given`,
`unboxed-iterator-lent-stepped-only` and
`unboxed-iterator-one-valued-refused` (`generic_facts.ts`) are caught. The
corpus, the crate corpus and itertools' Crater don't move.

## Amendment: an iterator JS holds as no object is given generic code in a place of its own (2026-11-21, #2255; Fixes #2253, #2254)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. This supersedes the refusal above. Generic code
holds an iterator type parameter, and a `&mut` of one, as the value
itself. An iterator of the crate's whose `&mut` is a cell, a fieldless
enum's string or a struct's number, is given it in a place of its own.

**`Form::Placed` (`generic_facts.rs`) is the crossing.** Its condition:
generic code's type is a type parameter whose `&mut` is the value itself,
or what an `IntoIterator` of its makes, `U::IntoIter` (ADR 0552). Its
caller's is such an iterator (`placed_iterator`). Then:
- Given by value or by `&`, it's `{ value: x }`. Given back, it's the
  place's `value`. A `&mut` of it is its caller's cell already, so a
  `&mut I` needs nothing.
- The crate's own steps of it, a `nth` or a `Box<dyn Iterator>`, take a
  place too (`own_steps_place`).
- Its dictionaries read the place (`placed_dictionary`,
  `traits/dictionaries/placed.rs`). Each entry is asked of `Form::Placed`
  for each type it takes and gives, in generic code's terms and its
  caller's. One that takes it reads `value`, `$throughBox`'s `1`. One
  that gives it back makes a place, a `clone()` a new one:
  `$throughPlace(triClone, { clone: [1] }, { clone: 1 })`. Its supertraits'
  and its associated types' bounds' entries are so too, `IntoIter`'s
  `Iterator`. A `&mut` to it where the trait declares a `&mut` its box
  holds, `&mut T`, is read and made too, as generic code wraps and reads
  an object's there (ADR 0454).
- Its `Copy` and `Clone` bounds take a dictionary, as a copy of the
  place is a new place (`analysis/mutation.rs`).
- Std's adapter of it, `Take<I>`, holds it in a place, as its `Iterator`
  impl does. So std's derived `Clone` of `Take<I>` holds its `I` so too
  (`held_in_place_by_its_impl`), and `cycle()` of a `take()` clones the
  place.

A unit struct is placed too, as only the instance knows it's one inside
generic code. Given by value, `{ value: undefined }`, so itertools' `get`,
`kmerge` and `kmerge_by` given `Panicking` no longer reach `$given` with
`undefined` (#2253). Its placed `Clone` gives a place, which `$assign`
writes through, so a hand-written `Clone` of one written back through
generic code's `&mut` runs (#2254). The runtime's helpers are unchanged.

**A field of the crate's own iterator is held in a place too
(`placed_field`).** A field whose declared type is a type parameter the
type's own `Iterator` impl holds in a place, `a: I` of `struct Both<I,
J>`, given such an iterator, is that place wherever the type is: `Both {
a: { value: "A" }, .. }`. So the impl, the type's other impls
(`held_in_place_by_its_impl` covers the crate's types and the type
itself) and what's generic over the type, a trait default building
`Both<Self, J::IntoIter>`, all hold it as the concrete code that builds
and reads it. Every read is at the place level: `mir_place` projects
the field through `.value` (representation's `held`), so a MIR read, a
discriminant (`match k.a`), a borrow and a write (`k.a.value = x`) are
right by construction. A `&mut` of the field is the place itself, kept
as a cell (`held_cell`), `step_one(pair.a)`. What reads or makes the
type by its shape asks the same answer through `held` and `holding`: a
copy (`copy_parts`, `moved_apart`, a kept copy, through `object_fields`)
makes a place of its own, `{ ...p, a: { value: p.a.value } }`; a derived
`Default`, `Ord` and `PartialEq` read and make it so; a tagged enum's
variants group only with variants whose fields are placed alike. A
tuple struct's array asks it by each part (`held_part`,
`holding_part`), `[{ value: "B" }, 1]`, and a parameter's pattern binds
what's in the place, `function blanksOf({ a: { value: a } })`. A
function that knows the type reads its fields by the same answer, so
`placed_within` asks only of a field whose type is generic code's
parameter whether that code holds it so. A
clone needs no rule of its own: a placed iterator with state is changed
in place, so its clone is made and given its place by `holding`; one
with no state, a unit struct, can't tell a shared place from its own.
Rejected: switching each site that reads a field (8 of them) to the
form, as a probe found 5 more wrong answers where code reads the struct
by its shape, and a discriminant read the place.

**Refused (`placed_within`):** a placed iterator inside what crosses,
where what holds it holds it otherwise. Examples: a `Vec<I>` or
`Option<I>` generic code makes and its caller reads; a field of the
crate's struct that isn't held in a place, a `Vec<I>` field or one its
`Iterator` impl doesn't hold so, which the caller builds; a `dyn` trait
of it, whose impl is the caller's. The call's arguments and result
refuse it, and so does an entry of a placed dictionary. Std's
iterators, `Take<I>`, are walked by what their `Iterator` impl holds in
a place, since only generic code makes them; the crate's own by their
fields, each held as `placed_field` says.

**A library's struct whose fields only its own code reads is walked by
its type arguments.** Itertools' `IntoChunks<I>` holds a
`RefCell<GroupInner<.., I, ..>>`. Its `GroupInner` is no iterator, so
walking its fields refused every `Itertools` entry given an iterator JS
holds as no object, 44 refusals in itertools' `tests/laziness.rs`. Only
the library's generic code builds and reads such a struct, and that code
holds a type parameter the same way everywhere, as std's iterators do.
So a struct of a crate that isn't std's or the program's, none of whose
fields the program can name (`is_accessible_from` the crate's root), is
walked by its type arguments, each held as generic code holds it. Each
argument is still walked, so a `Vec<I>` among them is refused. Still
walked by their fields:
- std's types, which the program reads by their shape;
- a library's enum, whose variants' fields are public;
- a library's struct with a public field, which the program may read by
  its shape, `open.inner` (`refused/library_public_field_holding_a_placed_iterator`).

**A tuple taken or given back by value crosses part by part
(`placed_shape`).** itertools' `multizip((a, b))` gives its
`From<(A, B)>` impl a tuple of two iterators. That impl holds each in a
place, `A: IntoIterator`. A tuple moved by value has no identity anyone
can see, so a new tuple of the parts, each in its place, is ADR 0262's
no-difference case: `[{ value: a }, { value: b }]` where the tuple is
written there, else `$inPlace(t, [1, 1])`, and `$fromPlace(r, [0, [1,
1]])` given back. A shape is `1` for a place, `0` for a part as it is,
and a list for a tuple's parts. `form_given` and `form_back` are its one
owner, so a call, a function value and a dictionary entry all take it.
`placed_within` takes the parts of such a tuple as crossing. A tuple
behind a `&` or a `&mut`, or inside a `Vec`, is still refused.

**A dictionary's entry takes what it's given as its own generic code
holds it.** A call adapts what it gives to the method it resolves to,
an impl's own or the trait's provided one (`call_callee`, `boxed_args`).
A dictionary's entry is that method, so a placed dictionary compares
what generic code holding the dictionary holds with what the method's
own generic code holds, per argument and result, and adapts only where
they differ:
- `$throughPlace` reads the places of the holder's, as before;
- `$intoPlace` makes the method's own, `$intoPlace(_A__B_Step(..), {
  step: [[1, 1]] }, { step: [1, 1] })`;
- where both hold it alike, it's given as it is: a blanket `impl<T:
  Iterator> Tools for T` given generic code bound by `Tools`, which used to
  read the place and give the default method the bare value.

A conversion's dictionary is written here of what it runs
(`conversion_dictionary`). Its entry takes what it's given as its caller
holds it, and gives the impl it runs that impl's forms
(`compiled_conversion`, `is_conversion`): `from: (value) =>
zipFrom_A__B__from($inPlace(value, [1, 1]), ..)`. Itertools' laziness
tests run 61 of 61, where main runs 58.

Corpus case `non_object_iterator_given_to_generic_code` covers a
fieldless enum and a unit struct with a hand-written `Clone`. It runs
them through std's adapters, `take`, `peekable`, a `cycle()` of a
`take()`, `zip`, `chain`, `flat_map`, `enumerate`, `skip` and `step_by`,
then a generic function bound by `Clone`, `Debug`, `PartialEq`,
`Default` and `Ord`, the crate's own `nth`, a `Box<dyn Iterator>`, and a
`&mut` given to a generic struct that clones and writes it back. It
matches native in every build and metamorphic rewrite; it also covers
the crate's `Both<I, J>` built by its own code and by generic code, its
field read, written and lent as a `&mut`, cloned, and a variant's,
`OneOf::It(I)`. Corpus case `iterator_field_in_its_place_copied_cloned_compared`
covers the shape owners: a copy of a `Copy` struct and enum, a clone, a
`vec![x; 2]`, derived `Ord`, `Hash`, `Default` and `PartialEq`, a
destructuring, `mem::replace` and `swap` of a field, a `match` and a
`matches!` of one, a unit struct with a `Drop`, variants of the same
field types held otherwise, a tuple struct copied, cloned, compared and
defaulted, and parameters' patterns. `trait_default_takes_an_iterator_as_its_callers_give_it`
compiles again. A `&mut` a variable keeps of a cell's `value` names the
cell (`fixed_kept`), so two snapshots read the handle a call gives back
as it is, and tidy's aliases drop the `r = r` of `r = &mut *r`. itertools'
`tests/laziness.rs` passes 61 of 61; its 3 lines in crater-decided.txt
and #2254's metamorphic-known lines are gone. Test
`test/diagnostics.test.ts`'s "a non-object iterator held otherwise
inside what crosses into generic code is refused" and the guide's
limitation hold the refusal.

## Amendment: a trait's associated iterator is its type's own value (2026-11-22, #2328, #2330)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. One case was a silent wrong answer, the other a
`TypeError`.

Generic code had a value of a trait's associated type bound by
`Iterator`, `type Out: Iterator`, as a JS iterator. The impl's dictionary
made one of what its methods gave, `make: (r) =>
$iterator(rangedMaker_make(r), $rangeNext)`, and its bound's entry stepped
that JS iterator, `OutIterator: () => ({ next: $next })` (ADR 0106's #972
and #1292 amendments). That gave one value two representations, and each
place it crossed from one to the other went wrong:

- **Given back to concrete code** (#2330): `fn made<T: Maker>(t: &T) ->
  T::Out { t.make() }` gave `main` the JS iterator. `main` reads
  `T::Out` as the impl's `Range<u32>`, and printed `undefined..undefined`
  where native Rust prints `3..6`.
- **Made by another of the type's bounds** (#2328): itertools'
  `HasCombination<I>` declares `type Combination: From<I> +
  Iterator<Item = Self>`, and `T::Combination::from(iter)` is the
  `From` entry's value, the impl's struct, which no entry made a JS
  iterator. `TupleCombinations::next` stepped it as one and threw `it.next
  is not a function`.

Unwrapping the JS iterator where it reaches concrete code was rejected,
because an iterator already stepped can't be turned back into a range.

**A trait's associated type bound by `Iterator`, of a trait that isn't
std's, is its type's own value wherever it's held: a range is a range, the
crate's own iterator is itself, std's compiled struct is itself, and std's
JS iterator is a JS iterator. Generic code steps it by the trait's
dictionary, `TMaker.OutIterator().next(it)`, or `$given` of that entry
where it needs a JS iterator, as it steps a type parameter given as it is
(ADR 0411). The entry is the type's own steps, `{ next: $rangeNext }` or
the crate's impl's `next`, and no entry makes a JS iterator of it.** A
method's `-> impl Iterator`, rustc's associated type of it, stays a JS
iterator (the #2005 amendment above). An opaque type can only be stepped,
so concrete code never reads its fields and it never needs two
representations. A
type declared by an extension alone, `type Out: DoubleEndedIterator`, has
an `Iterator` entry too, listed first, because an extension's dictionary
has no `next`.

- **One answer.** `assoc_iterator_as_it_is` (iterators.rs) answers it of
  the associated type. `held_by_generic_code` asks it of an impl's
  entries and methods. `projection_as_it_is` asks it of generic code's
  projection, as `iterator_given_as_it_is` and `extension_param` do, where
  one was taken for a JS iterator by its kind alone, `ty::Alias(..) =>
  true`. An architecture test bans that second decider.
- **std's `IntoIterator`'s `IntoIter` relates to this as follows.** It is
  the same question, answered by the same `projection_as_it_is`, with one
  real difference. A caller gives a `T: IntoIterator` that nothing holds
  as it is as a JS iterator, `$intoIter`'s (this ADR), so its `IntoIter` is
  that JS iterator unless the function holds it as it is (ADR 0552,
  `into_iter_held_as_it_is`). Another trait's associated value is only
  ever made by its impl's methods, as they make it, so it is always its
  type's own. A generic function that gives its `T::IntoIter` back to
  concrete code holds it as it is, and an impl's dictionary makes its
  `IntoIter` as its own value (ADR 0552's #2331 amendment).

The premise is tested by `test/associated-iterator-matrix.test.ts`. Each
shape of the type, a range, the crate's own iterator, std's JS iterator
and std's compiled struct, is stepped in generic code, given back to
concrete code that reads it as the impl's type, stepped and then given
back, stepped from its back, and made by a `From<I>` bound. Each runs as
a program and as a library against native Rust; `IntoIter`'s rows
pass since #2331. The pair `library_associated_iterator_flows_back`
covers a library's trait and generic code with its consumer's own impl.
Mutations: `assoc-iterator-a-js-iterator`,
`projection-js-iterator-to-generic-code`,
`projection-as-it-is-knows-its-back`,
`impl-makes-assoc-iterator-a-js-iterator` and
`assoc-iterator-extension-alone-no-iterator-entry` and
`assoc-iterator-impl-trait-in-trait-as-it-is`; two retargeted. This
supersedes ADR 0106's #972 and #1292 amendments where they make a JS
iterator of an associated type's value.

## Amendment: a provided method's `Self`, given its dictionary (2026-11-24, #2423)

#1962's "one given its dictionary is what that dictionary steps, as it is"
holds where the compiled body is given that dictionary too, a generic
function's `I: Iterator`'s. A trait's provided method compiled into
`@rust-js/std` isn't given one for `Self`: `iterator_partition(self, f,
BDefault, BExtendSelfItem, ..)` steps `self` as a JS iterator. So a type
parameter generic code holds with its dictionary, `w.0` of a `Wrap<L>`'s
or `it.into()` of an `impl Into<I>`'s, consumed by one, `partition` or
`unzip` into generic sides, `peekable`, was given the value itself, and
threw (`TypeError`) where native gives its sides. Now a value given its
dictionary to a compiled body that has no slot for that bound
(`takes_iterator_dictionary`, stepping.rs) is the JS iterator that
dictionary steps, `$given(w[0], LIterator)`, as generic code's own
consumers are given it, `$partition($given(w[0], LIterator), f)`. No new
form.

Results changed, so this follows ADR 0262's third case. Corpus
`provided_methods_of_a_dictionary_held_iterator_into_generic_sides`
(`partition`, `unzip` and `peekable` of a field's, `partition` of an
`impl Into<I>`'s) agrees with native; mutation
`dictionary-held-self-given-raw`.
