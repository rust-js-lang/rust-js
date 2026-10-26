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
