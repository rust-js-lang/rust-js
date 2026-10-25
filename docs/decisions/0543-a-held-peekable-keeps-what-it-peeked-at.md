# 0543. A `Peekable` a struct holds keeps what it peeked at

Status: Superseded in part by [0561](0561-std-bodies.md): a `Peekable` is
std's own struct, `{ iter, peeked }`, its `next`, its clone and its
`Debug` compiled from std's MIR, wherever it's made or held; its `IFused`
is what a compiled `Fuse` asks ([0566](0566-fuse-is-stds.md)). Extends [0164](0164-double-ended-iterators.md),
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
  `$fuse(it, IFused)`, kept as it ends where it's one. Where no caller
  says, an impl's parameter or a trait default's `Self`, it's `false`,
  cleared as before: `fused_fact` is the one answer.
- A `$closing` passes on what it closes' size hint, as it does its back
  and its length.

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

## Amendment: a fuse's `try_fold` of a `FusedIterator` keeps its source (#244)

`next` and `nth` were told the caller's `Fused` fact, but the compiled
`Fuse`'s `try_fold`, `$fuseTryFold`, wasn't. It always ran std's generic
`FuseImpl::try_fold`, which lets go of the source once it has run to its
end. For a `FusedIterator`, std's specialization keeps the source, so
it's dropped with the `Fuse`. The JS dropped it too early:
`take(1).fuse().any(|_| false)` dropped the rest of a `Vec` of values
with a destructor before `any` answered. `skip_while`'s and `find`'s
searches did the same through it (fuzz seed 51474). Following ADR 0262's
"results change" case, Rust's order is kept: `$fuseTryFold` takes the
fact, `SelfFused` in std's compiled `Fuse`, and keeps the source where
it's `true`.

The `fuse_fused_try_fold` corpus case runs `any`, `position` and `find`
against native Rust. The mutations `fuse-fused-try-fold-clears` (the
runtime) and `fuse-try-fold-fused-unsaid` (the std package's lowering)
are caught. Readability: the case's `Vec`s of values with a destructor
add `$iter` (77.3 to 77.6 in 10,000 lines) and renamed temporaries
(714.5 to 715.6) as each such case does. The output reads no worse.
