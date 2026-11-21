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

## Amendment: an iterator JS holds as no object is refused where generic code is given it (2026-11-21, #2254, #2255)

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
