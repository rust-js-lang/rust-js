# 1113. A body's `impl Trait` is revealed where its MIR is taken

Status: Accepted (m6-react's choice, 2026-11-10). Amends
[0061](0061-generic-iterators.md): what an `impl Iterator` hides is read
once, for a whole body, not by each rule that asks.

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
each was a wrong answer, so Rust's behavior is kept.

## Context

rust-js lowers a body from its analysis MIR (`mir_promoted`), where an
`impl Trait` a function gives back stays an opaque type in its callers'
locals: `fn queue() -> impl DoubleEndedIterator` is `Opaque(queue::{opaque})`
where `queue()` is called. ADR 0061 decided that an `impl Iterator` is
the type it hides, and each rule that asked what a value is had to call
`reveal` first: 104 calls did, and every rule that didn't answered of the
opaque type, as if the value were no iterator it knew.

A grid found 13 wrong answers of that one kind (#1142): each std adapter
and the crate's own iterator given back behind `impl DoubleEndedIterator +
ExactSizeIterator` or `impl Iterator`, times 23 consumers, 334 programs
against native Rust. The crate's own iterator's `enumerate().count()`,
`rev()`, `step_by(2)` and seven more threw `TypeError: value[0] is not
iterable` once it ended: its closing drop was given the `$iterator` wrapper,
as `is_user_iterator` and `iterator_given_as_it_is` asked the opaque type.
A peeked `peekable()`'s `enumerate().count()` dropped what it peeked at
first, where `Peekable::count` drops it last: `counts_by_compiled_count`'s
guard asked `enumerates_a_compiled_struct` of the unrevealed type. A
`map()`'s, an `inspect()`'s and a `rev()`'s `enumerate().rev()` threw
`enumerated.iter.back is not a function`.

## Decision

**`Mir::new` makes each `impl Trait` in a body, and in its promoteds, the
type it hides (`mir/revealed.rs`, `revealed`), as rustc's codegen reads
it after analysis.** It expands each opaque type with rustc's own
`try_expand_impl_trait_type`; one that holds itself, an `async fn` that
awaits itself through a box, stays as it is past where it does. So every
rule of a body's lowering asks the type its value is; a value its maker
gives and the one its caller holds are one representation, the maker's.

- **No rule of `mir/`, `iterators/` or `combinators/` reveals a type
  itself**: their 52 `reveal` calls are gone. A probe of `reveal` over the
  whole corpus and the grid, both builds, found that after the fold only
  six callers ever reveal a type, each of a THIR's or a signature's:
  `DropQuery::drops_in`, `is_user_iterator`, `back_stepped`,
  `asked_sources`, `is_array_iter` and `classify_conversion`.
  `Recognition::reveal` stays for those, types read elsewhere than a body's
  MIR. `test/architecture-iterators.test.ts` holds that `Mir::new` reveals,
  that nothing else expands an opaque type, and that none of those
  directories reveals one.
- **A held compiled std struct has a back only given it** (`has_a_back`),
  as `knows_its_back_beneath` already said (#856): a `Map` or a `Rev` given
  back behind `impl DoubleEndedIterator`, its type knowing its back, is
  `$knowing` of its compiled `next_back` where an `enumerate()` of it is
  stepped from its back. A back step's own test of std's struct, `rfind`'s
  of a `Chain` given back behind an `impl` (ADR 0920), is this one now,
  `as_double_ended`'s `knowing_held`: its mutation `std-struct-back-unknown`
  survived once the type was revealed, and with this rule taken out too
  (`compiled-struct-has-its-back`) `returned_impl_ends.rs` fails.
- **`counts_by_compiled_count` is `compiled_step`'s answer alone.** Its
  guard, that an exact-sized, double-ended source's enumeration is of a
  compiled struct (`enumerates_a_compiled_struct`, #712), is implied by a
  compiled `count` once the type is revealed, and it was the guard that
  missed the `impl`. Its mutations, `enumerate-reversible-compiled-count-taken`
  and `-unseen`, which #1109 found decided nothing else, go with it.
- What a library's function gives behind an `impl` is revealed in its
  consumer's MIR as in its own crate's; its `.d.ts`, which reads the
  function's signature, not a body, is as it was.

## Consequences

- Seven snapshots read better or the same: a step of an opaque-typed
  iterator through its compiled std step, `mapIterator_next(tens(v))`, is
  `$next(..)`, as it is of the same type without the `impl`; a call of a
  closure behind `impl Fn` that can't panic has no `try` to clean up after
  it; a temporary is named `p`, not `value$1`. One,
  `generic_chain_of_into`, holds its chain in a temporary, `const iter =
  chain(..)`, before its collect (#1154).
- The seven mutations that took one rule's `reveal` away,
  `kept-of-an-impl-unseen`, `made-closure-undropped`,
  `cloned-apart-unrevealed`, `knows-back-of-impl-hidden`,
  `iter-method-type-unrevealed`, `consumer-closes-unrevealed` and
  `opaque-stage-unseen`, are one now, `impl-trait-unrevealed`, which runs
  each of their cases.
- `size_hint()` of a held `filter()`, `skip_while()` or `take_while()` of
  items with a destructor is refused with or without an `impl`, in the
  default build; the grid's only refusals.

Held by `impl_trait_hides_std_adapters.rs` (seven adapters given back
behind an `impl`, each through every consumer of what it is),
`impl_trait_hides_own_iterator.rs` and
`impl_fn_that_panics_keeps_its_cleanup.rs` (a closure behind `impl Fn`
that can panic keeps the cleanup around its call, a plain one's, an
`Rc`'s and a `Box`'s), each against native Rust in both builds, and the
crates test `two crates: impl_iterator_of_a_library`. Mutations:
`impl-trait-unrevealed` (`lower/mir.ts`) and `compiled-struct-has-its-back`
(`lower/mir/iter/backs.ts`).
