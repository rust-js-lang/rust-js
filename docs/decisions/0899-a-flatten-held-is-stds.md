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
