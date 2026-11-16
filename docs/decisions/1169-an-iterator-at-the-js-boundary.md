# 1169. An iterator at the JS boundary

Status: Accepted (2026-11-17, the lead's decision; #1993, #1994, #1999).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behaviour is kept. An exported function's `impl Iterator` reached
JS as rust-js's own form, `{ start, end }` of a range, which `[...it]`
can't spread and whose `back()` a `DoubleEndedIterator` parameter calls,
so `lastOf(upto(4))` threw.

## Context

A function JS calls takes and gives iterators at the JS boundary:

- a parameter bound by `Iterator`, `DoubleEndedIterator` or
  `ExactSizeIterator`, which a JS caller gives an array or an iterator;
- an `impl Iterator`, or one of its extensions, which it gives back.

Three answers decided these apart. #1993 made a JS-facing function's
`DoubleEndedIterator` or `ExactSizeIterator` parameter the `$iter` of an
array a JS caller gives. The declarations printed an `impl` parameter as
`unknown`, a named `I: Iterator` as a bare type parameter `I`, and an
`impl Iterator` given back as `any` (#1994). And what the function gave
back was what its body reveals: a range's `{ start, end }`, std's
compiled `Filter` that knows no back (#1999).

## Decision

**One rule, `iterators/boundary.rs`, says what an iterator at the JS
boundary is,** from its bounds: what it gives, its `Item`, and whether it
knows its back, `DoubleEndedIterator`, and its length, `ExactSizeIterator`.
It answers of a JS-facing function's type parameter (`of_param`) and of
an `impl Trait` it gives back (`of_opaque`, `returned`). A type parameter
generic code is given as it is (ADR 0411), the crate's own value, is none.
Its three readers decide nothing of their own:

- **What a parameter takes.** Of `Iterator` alone, any of JS's iterables:
  the function steps `Iterator.from` of it (#1962). Of
  `DoubleEndedIterator` or `ExactSizeIterator`, an array, made its `$iter`
  where it's received (#1993), or the JS iterator that knows its back or
  its length, as it is.
- **What the declarations say** (#1994). A parameter: `Iterable<T>` of
  `Iterator`; `readonly T[] | DoubleEndedIterator<T>` or
  `ExactSizeIterator<T>` of its extensions, both intersected. What's given
  back: `IterableIterator<T>` of `Iterator`, else the same interfaces. A
  named `I: Iterator` is no type parameter of the declaration, as an
  `impl Iterator` isn't.
- **What's given back** (#1999). The function's own JS, the one its module
  exports, gives the JS iterator the boundary says: of a range,
  `$iterator(range, $rangeNext, false, $rangeNextBack, $rangeLen)`; of
  std's compiled struct asked its back, `$knowing(..)`; spread by `[...it]`.
  One already that iterator, std's compiled struct asked neither its back
  nor its length, is given as it is (`Returned::converts`).

**The crate's own Rust calls a copy** that gives what the body reveals,
`uptoRevealed`, which its callers read as a range's `{ start, end }`.
It's ADR 0051's copy, not a second mechanism: `JsGiven::Params` gains
`returns_js`, set in the function's own JS and never in a copy the
crate's Rust calls. A copy that differs only there is named `Revealed`.

Its own body's `return` alone converts. A closure lowered inside it, with
its item, `filter(|x| x % 2 == 1)`'s, gives back what it gives.

**Another crate's Rust calls a copy of its own.** A library's contract says
`ItemFact::ReturnsJs` of a function whose JS converts (library ABI 43), as
it says `Unboxed` of a parameter its JS takes otherwise: a consumer's Rust
call of it gives it more than its export takes (`instances::needed`'s
`gives_more`), so the consumer compiles its own copy, ADR 0914's, which
gives what the body reveals. One way for a library function's differing
needs to cross. A namespace's method called through a value, which no
copy of the consumer's can be, is refused as `gives_more` refuses it.

**`@rust-js/std` declares the two iterators,** named after the Rust
traits they mirror, structural, each extending `IterableIterator<T>`:

```ts
export interface DoubleEndedIterator<T> extends IterableIterator<T> {
  back(): IteratorResult<T>;
}
export interface ExactSizeIterator<T> extends IterableIterator<T> {
  len(): number;
}
```

**`back()` and `len()` are public API** from here on, beside `next()`:
an exported function gives one back, a JS caller calls them, and another
exported function takes it. The runtime's `$iterator` and `$knowing` say
so where they're defined. `back()` gives the last item left, as `next()`
gives the first; `len()` how many are left.

## Consequences

- `boundary.rs` is the one owner. `declarations.rs`, `js_given.rs` and
  the lowering's `return` ask it; a library's contract carries its answer
  to consumers as `ReturnsJs`, which `instances::needed` reads.
- Crate pair `library_impl_iterator_given_back` holds a library's
  `upto`, generic `counted` and `Bag::odds` called from another crate's
  Rust, from each end, by length and named as a value, against native, and
  from JS, `[...upto(4)]`, `back()`, `len()`; `impl_iterator_of_a_library`
  and `iterators_stepped_from_their_back_across_crates` too.
- A function that gives back an `impl Iterator` of std's compiled struct,
  asked neither its back nor its length, gets no copy: its JS is the
  boundary's already (`generic_clones_of_unseen_callers` is unchanged).
- Held by `test/declarations-iterators.test.ts`, each printed shape and a
  strict TypeScript check of the round trip and of what's refused, and by
  `test/lowering-iterators.test.ts`'s "given back to JS", JS callers of
  `upto`, `evens` and `counted`, their `back()`, `len()` and `[...it]`,
  and the crate's own call of `uptoRevealed`. Mutations of each reader's
  use of the rule and of `returns_js` (`iterators/boundary.ts`,
  `declarations.ts`, `analysis/js_given.ts`).
