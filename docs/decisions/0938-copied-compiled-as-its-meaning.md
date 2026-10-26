# 0938. `Copied` is compiled as its meaning, and a type parameter whose copy is itself is given no copy function where std's code asks one

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0932](0932-an-adapter-compiled-as-its-meaning-alone.md) and
[0289](0289-copy-bounds.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior is
kept where rust-js refused it. With std's structs alone, a `copied()`
stepped from its back was refused, "the `DoubleEndedIterator` of a
`Copied<slice::Iter<'_, Point>>`".

## Context

A `copied()` is its source's JS iterator in the normal build (ADR 0480),
which conflicts with std's struct; compiled as its meaning alone (ADR
0932), it's std's struct only in the build of std's structs. There, std's
compiled `Copied` asks a `T: Copy` dictionary, which a generic function of
the crate's bound `V: Copy` wasn't given: every value its callers give is
its own copy, so it takes no copy function (ADR 0289), and asked for one
it was refused, "Copy without representation evidence".

## Decision

- **`Copied` is `MEANING_ONLY`**, compiled whole.
- **A type parameter whose `Copy` bound takes no copy function is given
  none, `undefined`, where code asks its `Copy`**, as a type known whose
  copy is itself is (ADR 0289, amended).

## Consequences

- ADR 0908's `differs` is 7, from 8: `copied_structs_apart.rs` runs as
  native Rust does with std's structs alone; it and
  `heap_pop_of_a_param.rs` join the suite's few.
- The normal build's JS doesn't change.
- Mutations: `copied-back-uncompiled`, `identity-copy-param-refused`.
