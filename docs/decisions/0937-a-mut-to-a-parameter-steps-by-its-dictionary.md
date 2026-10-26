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
