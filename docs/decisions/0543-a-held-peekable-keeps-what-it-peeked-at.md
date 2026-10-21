# 0543. A `Peekable` a struct holds keeps what it peeked at

Status: Accepted. Extends [0164](0164-double-ended-iterators.md),
[0466](0466-a-generic-maps-class-is-its-callers.md), [0473](0473-fuse-is-rusts-fuse.md) and
[0542](0542-a-held-adapter-keeps-its-source-and-state.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a held `Peekable`'s
`Debug`, its clone and its size hint are Rust's, and so is whether a
generic iterator's `Fuse` keeps it as it ends.

## Context

itertools' `WithPosition` holds a `Peekable<Fuse<I>>`; its `Debug`, bound
by `Peekable<Fuse<I>>: Debug`, shows `Peekable { iter: Fuse { iter:
Some(..) }, peeked: Some(Some("b")) }`, and its clone clones what it
peeked at. Both were refused. A `Peekable` of an array's iterator was a
`$iter` that peeks at its array, which keeps no `peeked` to show. And a
generic iterator's `Fuse` cleared it as it ended (ADR 0473), where Rust's
keeps a `FusedIterator`, `Fuse { iter: Some(2..2) }`.

## Decision

**`Peekable` is the second held kind (ADR 0542): wherever a crate whose
types hold one makes one, it's `$peekable(source)`, which keeps what it
steps, `iter`, and what it peeked at, `peeked`.**

- Its `Debug` is std's, `Peekable { iter: .., peeked: .. }`: `None`,
  `Some(None)` of a peeked end, or `Some(Some(x))`, plain and pretty.
- Its clone is `$peekableClone`: of a clone of what it steps, and of what
  it peeked at, an item's clone.
- Its size hint is Rust's: its source's, and one more for a peeked item,
  `(0, Some(0))` after a peeked end; any `$peekable`'s.
- **A type's `Debug` a bound gives, `Peekable<Fuse<I>>: Debug`, is that
  bound's dictionary's**, as rustc resolves it, before what rust-js
  writes of the type.
- **Whether a type parameter is a `FusedIterator` is its caller's fact,
  `IFused`**, as `TSort` is (ADR 0471): generic code's `fuse()` is
  `$fuse(it, IFused)`, kept as it ends where it's one. Of an impl's
  parameter, given no fact, it's cleared as before.

## Why

`held_peekable` is `WithPosition`: its `Debug` plain and pretty before and
after a step, a clone where it is stepped apart, size hints, and a range
whose `Fuse` keeps it; and a struct holding a `Peekable` of an array's
iterator shows what it peeked at, against native Rust.

## Consequences

- itertools' `WithPosition` compiles; three refusals remain: `Cycle`'s
  and `PutBack`'s `Debug`, and the map folded from its back in a library.
- A crate whose types hold a `Peekable` makes each `peekable()`
  `$peekable`, an array's too: `interpreter`'s lexer reads
  `$peekable($iter(..))`.
- Generic code's `fuse()` takes `IFused`: `pair(i, IIntoIterFused)`.
- Mutations make a held one peek at its array, refuse its `Debug` and its
  clone, read no bound's `Debug`, give `fuse()` no fact, ask an impl's,
  forget what it peeked at in a clone, and leave it out of the hint.
