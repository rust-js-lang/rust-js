# 0937. A `&mut` to a type parameter steps from its back by the dictionary the parameter was given

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0928](0928-a-mut-i-folds-from-its-back-by-try-rfold.md) and
[0912](0912-a-std-struct-folds-by-its-own-compiled-fold.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior is
kept where rust-js refused it. With std's structs alone,
`self.inner.by_ref().map(f).rfold(..)` of an `R: DoubleEndedIterator` was
refused, "the `DoubleEndedIterator` of a `R`".

## Context

std's compiled `Map::rfold` of a `&mut R` asks a `&mut R`'s
`DoubleEndedIterator`. A `&mut I`'s `Iterator` dictionary is `I`'s, the
entries std's impl for `&mut I` passes on (ADR 0912); an extension's was
built from the type alone, which of a type only its bounds know has none.

## Decision

- **An iterator extension's dictionary of a `&mut` to a type only its
  bounds know is of the dictionary that type was given**: `next_back`,
  `nth_back` and `try_rfold` of a `DoubleEndedIterator`, `len` of an
  `ExactSizeIterator`, what std's `&mut I` passes on; not `rfold` (ADR
  0928). It and the `Iterator`'s keep entries one way,
  `entries_passed_on`.

## Consequences

- ADR 0908's `differs` is 8, from 9: `held_enumerate.rs` runs as native
  Rust does with std's structs alone, and joins the suite's few.
- The normal build's JS doesn't change.
- Mutations: `mut-ref-extension-of-evidence-unseen`; `mut-ref-dictionary-whole`
  follows its line.

## Amendment: what a `&mut` and a `Box` pass on, by the catalog (#2000, #2075, 2026-11-18)

Two readers of what std's `&mut I` and `Box<I>` pass on to their iterator
didn't ask `PASSED_ON`, the catalog the rest read (`Wrapper::passes`):

- `mut_ref_extension_dictionary` kept its own list, `nth_back` and
  `try_rfold` past its entry. Its names are now the dictionary's own past
  its entry, `BACK_ENTRIES` (`rfold`, `nth_back`, `try_rfold`, which
  `iterator_extension_dictionary` asserts it makes alone), that the
  wrapper passes on as `I`'s own: of a sized `I` the same two as before,
  no `rfold` (ADR 0928); of an unsized one, `nth_back` alone, where the list
  gave its `try_rfold` too, which std's `&mut I` of an unsized `I` doesn't
  pass on.
- `as_exact_size` and `as_double_ended` refused a `&mut` to a range given
  to generic code bound by `ExactSizeIterator` or `DoubleEndedIterator`
  (#2075). What no form of theirs knows is made so as what it lends is,
  where the catalog passes `len`, and `next_back` where it's asked, on to
  it (`passed_on_to`): the range is stepped where it is, `$iterator(r,
  $rangeNext, false, $rangeNextBack, $rangeLen)`, so its owner sees each
  step. Only what was refused changes; a `&mut` to the crate's own iterator
  keeps its own arm. Not a `Box`'s, which stays refused (#1892): std's
  `Box<I>` isn't `TrustedLen` where `&mut I` is, and lifting the refusal
  gave `boxed_range_taken_through_its_borrow` a `Vec` of capacity 3 where
  native's is 4.

Corpus case `lent_range_made_knowing_for_generic_code`: a range partly
stepped before it's lent, counted and stepped from its back through the
`&mut`, then from its front by its owner, and a `RangeInclusive`, against
native. Mutations `lent-range-length-unknown` and `lent-range-back-unknown`;
`mut-ref-extension-of-evidence-unseen` re-anchored. The row that holds the
derived names, a held `&mut R` of the crate's iterator folded from its back
through std's `Map`, waits on #2104, whose plain build gets it wrong.
