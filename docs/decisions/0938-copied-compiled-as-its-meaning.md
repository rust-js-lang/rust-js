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

## Amendment: `Cloned` is compiled as its meaning too (#727)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior is
kept where rust-js refused it. With std's structs alone,
`v.iter().cloned().zip(once(7).chain(once(8))).count()` was refused,
"reading a `Cloned<slice::Iter<D>>` by its index, as std's zip of it
does": std's compiled `Zip` asks whether each side is
`TrustedRandomAccess` (ADR 0975), which a `Cloned` of a slice's items is,
and std's `Cloned` wasn't compiled, so there was no reader of its item at
an index, and a random-access fact of a type rust-js can't read by index
is refused, never given.

- **`Cloned` is `MEANING_ONLY`, compiled as `Copied` is**: its `next`,
  `__iterator_get_unchecked`, `size_hint`, `fold`, `try_fold`,
  `next_back`, `rfold`, `try_rfold`, `len`, derived `clone` and `fmt`, and
  std's `cloned`. Those are all its impls write; its `nth`, `count` and
  the like are `Iterator`'s, of its `next`. Its dictionary carries its own
  reader by index, `getUnchecked`, as `Copied`'s does, so std's zip reads
  it by index where both its sides are random access, cloning each item
  as it reads it, and steps it by `next` where the other side isn't.
- **Not the fact asked of a zip's sides together.** Asking
  `TrustedRandomAccess` of both sides at once would leave this case, its
  other side a `chain()`, stepped by `next`; but a zip of a `cloned()` and
  a slice's items, or a range, both random access, would still be
  refused. Compiling std's `Cloned` covers both, and keeps the one rule:
  a random-access type rust-js has no reader for is refused.

Consequences: the iterator forms' `differs` is empty again;
`adapters_own_work_that_panics.rs` checks a `cloned()` zipped with a
range and with a slice's items, read by index, a clone panicking part-way,
and folded, against native Rust in both builds, and joins the suite's
few. The normal build's JS doesn't change: there a `cloned()` is still
what the recognizers make of it. Mutations: `cloned-reader-uncompiled`,
`cloned-compiled-everywhere`.
