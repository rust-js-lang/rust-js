# 0928. A `&mut I` folds from its back by `I`'s `try_rfold`, never `I`'s own `rfold`

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0912](0912-a-std-struct-folds-by-its-own-compiled-fold.md) and
[0487](0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, `(&mut it).rev().sum()` of the crate's iterator
called its own `rfold`, where Rust's `&mut I` calls `I`'s `try_rfold`,
std's default, by `next_back`: the program printed what its `rfold`
prints, which native Rust doesn't.

## Context

std's `impl DoubleEndedIterator for &mut I` passes `next_back`,
`advance_back_by` and `nth_back` on to `I`, and folds from its back by
`spec_rfold`, which for a sized `I` is `I`'s `try_rfold`: `I`'s own `rfold`
isn't called. ADR 0912 found the same of `fold`, and gave a `&mut I`'s
`Iterator` dictionary `I`'s `next`, `size_hint` and `nth` alone.
`iterator_extension_dictionary` took a `&mut I` for `I`, its `rfold` too.

## Decision

- **A `&mut I`'s `DoubleEndedIterator` dictionary is `I`'s `next_back` and
  `nth_back`, not its `rfold`**: what folds it from its back steps it by
  `next_back`, as std's `try_rfold` does.

## Consequences

- ADR 0908's `differs` is 16, from 19: `rev_own_back_methods.rs`,
  `own_iterator_dropped_from_back.rs` and
  `own_iterator_dropped_through_adapters.rs` run as native Rust does with
  std's structs alone; `rev_own_back_methods.rs` joins the suite's few.
- The normal build's JS doesn't change.
- Mutations: `lent-rfold-passed`.

## Amendment: a type parameter's given as it is, lent (#994)

`held.by_ref().sum()` of a type parameter given as it is gave the sum
`$given(held, IIterator)`, which carries the crate's own `fold`: the sum
ran it, which took the iterator, printed what it does and dropped it
before the sum was done, where std's `&mut I` steps it by `try_fold`'s
`next` and it's dropped where its function ends. A `&mut` of one given its
dictionary is lent, `$lent($given(..))`, as the crate's own iterator
through a `&mut` is (`as_js_iterator`). Corpus case
`by_ref_of_held_iterator`.

## Amendment (2026-11-16, #1986, #1991): `$lent` folds by `try_rfold` too, and an unsized `I`'s `&mut` takes std's defaults

Case C again: results change. `$lent`, the runtime's one owner of what a
`&mut I` passes on, gave `try_rfold` but no `rfold`, so `$foldBack` of a
lent iterator fell to `$foldBy(it, "back")`, std's default `rfold` by
`next_back`. A `take()` lent through `pass(a.by_ref())` then dropped what
it skips 4 then 5, by `Take`'s `nth_back(len - n)`, where std's `&mut I`
folds by `Take`'s `try_rfold`, 5 then 4 (#1986). And a `&mut dyn
DoubleEndedIterator` was lent as a sized one: through a compiled struct's
proxy its `rfold` was the struct's own, 5 then 4, where std's `&mut I` of
an unsized `I`, the `default` of `DoubleEndedIteratorRefSpec`, steps by
`next_back`, 4 then 5; of a `skip()`, the item it skips was dropped inside
the fold, where native drops it at the next `next()` (#1991).

- What a `&mut I` passes on is data, `$passedOn` in src/runtime/lent.js:
  `lent`, of a sized `I`, its `next`, `size_hint`, `advance_by`, `nth`,
  `try_fold`, `next_back`, `advance_back_by`, `nth_back`, `try_rfold` and
  `len`, its `fold` and `rfold` by `I`'s `try_fold` and `try_rfold`; and
  `lentUnsized`, of a `dyn`, the same but none of the four folds. Each step
  passed on is read of `I` when it's asked, so a compiled struct's back
  `knowing` gives it after it's lent is passed on too; a step it doesn't
  pass on is never read through a compiled struct's proxy.
- The compiler says which: `lent_iterator(it, by)` of the `&mut` type,
  `$lent(it)` of a sized referent, `$lent(it, "lentUnsized")` of an
  unsized one (`iterators::Lent`); one lent again by `$slicing` keeps its
  kind.
- The lists are std's facts: test/passed-on.test.ts reads std's `impl ..
  for &mut I` and `impl .. for Box<I>`, and their specializations, in this
  toolchain's rust-src, and fails where `$passedOn`, or the compiler's
  `BOX_PASSES_ON` (#1601), differs.

A `Box` of a compiled std struct folded from its back still runs the
struct's own `rfold` (#1995), and with std's iterator structs a `rev()` of
a `&mut dyn` steps a `Vec`'s back without its slice steps (#1996): each
its own issue. Corpus case `lent_iterator_folded_as_std_passes_it_on`,
against native in both builds; copy matrix cells 217, 2421, 38013, 40011,
42009, 44007, 46005, 48003 off the known list. Mutations
`lent-rfold-by-its-back`, `lent-unsized-folds-its-own`,
`lent-struct-steps-of-its-struct` and `lent-dyn-as-sized`; the earlier
`lent-*` mutations retargeted to the lists.

## Amendment (2026-11-17, #1995): a `Box` of std's compiled iterator takes the traits' defaults, as std's `Box<I>` does

Case C: results change. `passed_through_box` called a trait's default on
a `Box<I>` as `I`'s for any std iterator, "a default's too". So
`Box::new(take).rfold(..)`, and a `rev()` of it folded, ran `Take`'s own
`rfold`, dropping what it skips 5 then 4. std's `impl DoubleEndedIterator
for Box<I>` writes only `next_back` and `nth_back`, so its `rfold` is the
trait's default by `next_back`: 4 then 5.

- A `Box` of std's compiled iterator passes a call through to `I` only
  where std's impl writes it (`std_impl_writes`), as a `Box` of the crate's
  iterator already did (#1601). Any other, a default, stays the box's.
- Stepped from its back (`as_double_ended`), it's `$lent(it, "boxed")`:
  `I` made knowing its back, passing on only what `Box<I>` passes on, and
  closing `I` as it closes, since a box owns what it holds.
- With std's iterator structs, its extension dictionary gives its
  `next_back` and `nth_back`, `I`'s compiled ones, and no fold of `I`'s.

`$passedOn`'s `boxed` and `boxedUnsized` lists are held to std's
`impl .. for Box<I>`, and to `BOX_PASSES_ON`, by test/passed-on.test.ts
until #2000 makes them one catalog. Corpus case
`boxed_iterator_folded_as_std_passes_it_on`, against native in both
builds, in test/iterator-forms.test.ts's few. Mutations
`box-compiled-default-passed-through`, `box-compiled-back-its-own` and
`box-compiled-dictionary-unboxed`; the box mutations of std_items.rs,
`compiled-struct-rfold-as-js` and `double-ended-of-a-js-iterator-again`
retargeted.
