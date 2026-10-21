# 0549. A held `Fuse` is stepped from its back as its use site knows it

Status: Superseded by [0566](0566-fuse-is-stds.md): a `Fuse` is a compiled
std iterator, given its compiled `next_back` and `len` by a use site in
this decision's form, `$knowing`, of the struct itself; a held
`Enumerate` is still this decision's. Extended [0542](0542-a-held-adapter-keeps-its-source-and-state.md)
and [0528](0528-fuse-and-enumerate-from-either-end.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a held adapter is
stepped from its back where Rust's is, wherever it was made.

## Context

itertools' `PadUsing` holds a `Fuse<I>`, made by `pad_using` where `I` is
only an `Iterator`, and steps it from its back, and counts it, in an impl
where `I: DoubleEndedIterator + ExactSizeIterator`. The `$iterator` made of
`I` was given no back and no length, as none was given where it was made,
and `$fuse` took its own back and length from its source's as it was made:
`pad.iter.len is not a function`. A held `Enumerate` was given a use site's
by `$knowing` (ADR 0542); a `Fuse` wasn't, as its type was taken to know
its back wherever its source's dictionary is given.

## Decision

**A held `Enumerate` or `Fuse` of a source given as it is is made knowing
its back and its length where it's stepped from its back or counted, by
that use's dictionaries, `$knowing(pad.iter,
IDoubleEndedIterator.next_back, IExactSizeIterator.len)`, wherever it was
made.**

- `held_steps_source` is the one answer to which held adapters a use site
  makes knowing, and `knowing_held` the one way it does, which
  `as_double_ended`, `as_exact_size`, `next_back()` and `len()` ask first.
  Either dictionary may be missing: its entry is `undefined`.
- `$fuse`'s back and length are what it steps', asked as they're stepped,
  not copied as it's made; `$closing` keeps what it closes, `iter`, for
  `$knowing` to reach.

## Why

`held_fuse_back` makes a `Fuse` where only `Iterator` is known, of an
array's iterator, of the crate's iterator with a destructor and of a
range, and steps each from its back and counts it, against native Rust.

## Consequences

- itertools' `pad_using` steps from its back.
- `$knowing(..)` is written at each use, twice in `PadUsing`'s
  `next_back`. The [std-bodies draft](draft/std-bodies.md) would retire
  held adapters' hand-modeling for std's own bodies.
- Mutations leave the use site's dictionaries ungiven, a `Fuse` off the
  list, `$closing`'s source unreachable, and `$fuse`'s ends fixed where
  it's made.
