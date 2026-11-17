# 1171. The crate's own `IntoIterator` taken as any `IntoIterator`

Status: Accepted (2026-11-17, the lead's decision; #1940). Supersedes ADR
0160's refusal of a collection of the crate's given where any `IntoIterator`
goes.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behaviour is kept. It was refused, and idiomatic Rust wants it: `impl
IntoIterator for &Board`, then `v.extend(&board)`, `a.iter().zip(&board)` or
`fn f(items: impl IntoIterator<Item = u32>)` given `&board`.

## Context

Std's recognized code and the crate's generic code iterate what they're
given as JS does. Generic code's `items.into_iter()` is `$intoIter(items)`,
and the runtime's `$extend`, `$lazyZip`, `$lazyChain` and `$splicing` take
its `[Symbol.iterator]()`. A collection of the crate's is a plain object or
a class, so JS iterating it would never call its own `into_iter`, and
Rust's `into_iter` has an observable time: std's `zip` and `chain` call it
as they're made, `extend` as it starts, and `splice` after `drain` has
taken out its range.

## Decision

The crate's own `IntoIterator` taken as any, by what iterates it as JS
does, is `$ownIntoIter(() => boardIntoIterator_into_iter(board))`: an
iterable whose `[Symbol.iterator]` is its own `into_iter`, made when what
takes it asks for its iterator, which is where Rust calls `into_iter`.

- One maker, `own_into_iter_of` (traits/calls.rs). One asker,
  `iterable_items`, the owner of "the items of an `IntoIterator` a function
  takes as JS steps them". It asks after the dictionary's case (ADR 0457).
  Std's recognizers reach it, and so does the crate's or a library's
  generic code, through `given_iterators`.
- Std's bodies compiled for the build of std's structs are given the
  collection's `IntoIterator` dictionary, whose `intoIter` is its own
  `into_iter`, as before, and take the value as it is.
- The refusal and its decider, `collection_as_iterable`, are gone.
- `$splicing` makes its replacement's iterator after the range is checked
  and taken out, as std's `splice` makes its `Drain` first. Where that
  `into_iter` panics, what was taken out is dropped and the tail put back.
  A `splice` of the crate's own `IntoIterator` takes `$splicing`, never
  `$splice`, which puts its replacement in with no unwind.
- With std's iterator structs, the impl's dictionary gives its `IntoIter`
  as JS's iterator, `{ next: $next }`, with no `getUnchecked`. So a call
  whose evidence says that `IntoIter` is `TrustedRandomAccess(NoCoerce)`,
  a zip's, is refused, keyed on that fact wherever it's asked, naming #2020.

The name: `$intoIter` was already generic code's `into_iter` of what its
caller gives, so the new helper is `$ownIntoIter`.

## Proof

- Corpus `own_into_iterator_given_where_any_goes` gives `&Board` and `Board`
  to:
  - a `chain`, with a print between its making and its first step;
  - an `extend` of a `Vec`, a `HashSet`, a `BTreeSet`, a `VecDeque` and a
    `HashMap`;
  - generic `impl IntoIterator` code, between its own prints;
  - a `splice` in range, out of range (no `into_iter`, as native) and with
    an `into_iter` that panics (`[1, 4]`);
  - a by-value one of items with a destructor;
  - an `extend` whose `into_iter` panics.

  It matches native in the plain build, with std's iterator structs, and
  through every metamorphic rewrite.
- Corpus `own_into_iterator_zipped` (a zip, with a print between its
  making and its first step, and a panicking `into_iter`) matches native in
  the plain build. It's in `iterator-forms`' `differs`, refused with std's
  structs (#2020), as a diagnostics row holds.
- Mutations, each caught:
  - `own-into-iter-unmade`
  - `own-into-iter-eager`
  - `splicing-into-iter-before-drain`
  - `splice-own-into-iter-put-eagerly`
  - `own-into-iter-indexed-unrefused`

## Consequences

The crate's collections are given as Rust gives them, `into_iter` at
Rust's point. #2020 makes the impl's `IntoIterIterator` entry its
`IntoIter`'s own Iterator dictionary, after which the std build's refusal
goes.
