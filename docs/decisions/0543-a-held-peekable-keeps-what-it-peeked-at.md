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

## Amendment: what holds `fused-fact-unanswered` (2026-11-11)

Since 31d29cf63 a copied default is given its caller's `TFused`, so
`fuse_size_hint`'s copy no longer reaches a type parameter's `Fused` no
caller says, and mutation `fused-fact-unanswered` survived on main. Such a
fact is still asked, and answered `false`, by corpus cases
`flat_map_of_a_where_clause_into_iterator` and `associated_iterator_steps`,
which the mutation refuses ("`Fused`"); they hold it now.

## Amendment: a fact no caller gives is refused, never `false` (#2042, 2026-11-18)

The Decision's "where no caller says ... it's `false`" was a guess, and it
gave a wrong answer. An impl's method that steps a `Fuse<T>` it holds from
its back, `a.by_ref().rev().fold(..)` as itertools' `ZipLongest::rfold`
does, makes the held struct knowing its back (`knowing_held`): it calls
`Fuse`'s compiled `next_back`, which asks `T`'s `Fused`. The analysis
never saw that call, which is lowering's, not the body's, so no caller gave
the fact and it was `false`. A `FusedIterator` source was dropped at its
first `None` and never asked again, where native asks it again and drops
it at the end. Results changed, so Rust's behavior is kept (ADR 0262).

One catalog now, `KNOWING_STEPS` beside `STRUCT_STEPS`: the extension
methods a held compiled struct is made knowing with (`next_back`, `rfold`,
`nth_back`, `try_rfold`, `len`). Lowering's makers of those steps assert
each is in it (`is_read_step`). The analysis asks what each asks of every
compiled struct a call is given where the callee may step it (a parameter
bound by `Iterator`, an extension of it or `IntoIterator`), as it asks
`STRUCT_STEPS` of a struct a body makes, of a body read from its THIR and
of one read from its MIR alone, a library's impl its consumer compiles,
itertools' `ZipLongest::rfold` among them. Any specialized impl such a step
reaches, today `FuseImpl`'s, is asked without a walk of its own. It skips
the struct whose own method the call is, `Fuse`'s `rfold` or `len` of it
as it is, whose compiled body asks what it asks.

With every asking body given the fact, the fallback is a refusal like the
others ("whether it's a `FusedIterator` of `T`, which no caller gives"). The
two corpus cases of the amendment above are given it now, and itertools
compiles as before. Corpus case
`fused_fact_of_an_impls_parameter_asked_through_std`, a `FusedIterator`
source and a plain one, each holding what drops, through an impl's
`rfold`, against native (it differed at main); and pair
`fused_fact_of_a_librarys_impl`, the same impl a library's, which a first
version, asking a THIR body's calls alone, refused in the consumer, as it
did itertools' probe (the push's crate corpus). Mutations
`knowing-steps-unasked`, `knowing-steps-of-mir-bodies-unasked`,
`knowing-steps-of-own-method` and `knowing-steps-of-any-argument`; the last two each move a snapshot by an
`IFused` nothing reads, `map_stepped_from_its_back`'s and `std_fuse`'s.
`fused-fact-unanswered`, which held the two cases' `false`, is removed:
they're given the fact, and no known program reaches the refusal, which is
the guard of one the analysis misses, as the other facts' are.
