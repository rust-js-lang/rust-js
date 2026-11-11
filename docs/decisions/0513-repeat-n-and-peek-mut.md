# 0513. `iter::repeat_n` and `Peekable::peek_mut`

Status: Accepted. Extends [0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md) and
[0071](0071-stepping-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): how many clones are
made, and when a value drops, would change, so Rust's is kept.

## Context

`iter::repeat_n(x, n)` and a `Peekable`'s `peek_mut()` were refused: rust-js
had no form for either. And `$repeating`, `repeat`'s, was written inside
`Iterator.from(..)`, though it makes a JS iterator already.

## Decision

**`repeat_n(x, n)` is `$repeatingN(x, n, clone, drop)`**: `n - 1` clones,
then `x` itself, as std's moves it last; of a count of none, `x` dropped
where it's made, as std's drops it then; and dropped when it's closed
before its last.

**`peek_mut()` is a `&mut` to the item a `Peekable` gives next**: of an
object, the item itself, `$peek(it)`; of a number or text, a handle on it,
`$peekCell(it)`, on its array's place, `$mutAt(items, at)`, or on the step
a lazy one keeps for its next `next`.

`$repeating` and `$repeatingN` are in `ITERATOR_HELPERS`, which the
runtime's test holds, so neither is written inside `Iterator.from`.

## Why

- **It's the same program**: `repeat_n_and_peek_mut` repeats numbers,
  strings and a value whose `Clone` prints, none, all and some of them,
  and writes through `peek_mut` of an array's iterator, a lazy one and
  strings, against native Rust.

## Consequences

- `iterator_sources` and `repeat_drops` lose their `Iterator.from`.
- Mutations drop no value of a count of none, clone the last, drop none
  when closed, write a lazy step's copy, give a number for a `&mut`, and
  take `repeat_n` for `repeat`.

## Amendment: `Repeat`'s and `RepeatN`'s own steps, which clone none of what they pass (2026-11-10, #1124)

`repeat(x).take(n).count()` cloned `x` `n` times and dropped each clone,
where native Rust makes none. Std's `Take::count` asks its source's
`advance_by(n)`, and `Repeat::advance_by` is a no-op (repeat.rs:92). It
isn't a specialization: `Repeat` and `RepeatN` write steps of their own,
which `$repeating` and `$repeatingN` didn't, so `$Steps`' defaults, by
`next`, cloned each item they passed. Both builds did this, and so did
`skip(n)` and `step_by(n)` over them, `repeat(x).nth(k)`, and
`repeat_n`'s `nth`, `last`, `count` and `skip`.

**Each writes std's steps of its own:**

- `$repeating`: `advanceBy`, a no-op, so `nth` is one clone (repeat.rs:92,
  :99).
- `$repeatingN`: `advanceBy`, which counts down and past its end drops the
  value, so `nth` clones only the item it gives; `last`, the value itself;
  `count`, its length, the value dropped with it (repeat_n.rs:140–165).

Each says so with `$owns`, the runtime's own facts, so `$skipping`,
`$stepBy`, `$lastDropping` and `$count` ask it. The lowering's catalog of
own steps (`own_steps.rs`) names them too, as std sources
(`own_source_steps`, `source_writes`), so a `count` of a `repeat_n` is
`$count`, its own, never a `reduce`.

Left as refusals, no wrong answers:
- the back steps of `take(n)` of a `repeat`, and of a `repeat_n`, in the
  default build;
- `repeat_n(..).len()`.

Held by `test/corpus/repeat_own_steps_clone_none.rs`, each clone and drop
printed, against native Rust in both builds: through `take`, `skip`,
`step_by` and `fold`, held, in generic code, and stepped to its end by
`next`. Mutations `repeat-advance-clones`, `repeat-n-advance-clones`,
`repeat-n-own-last-cloned`, `repeat-n-count-steps` and
`repeat-own-nth-unflagged` (`runtime/repeating.ts`),
`repeat-n-own-steps-unknown` (`lower/recognition/own_steps.ts`) and
`source-own-count-unasked` (`lower/mir/iter.ts`); `repeat-n-last-cloned`
held by it too.

## Amendment: `Repeat`'s `count` and `last` panic (2026-11-10, #1138)

`repeat(x).count()` and `last()` ran forever, std's defaults of `next`,
where `Repeat`'s own panic, "iterator is infinite" (repeat.rs:110, :120):
a hang where Rust panics, which `catch_unwind` and `#[should_panic]` see.
The amendment above left them as they were; that was wrong.

**`$repeating` gives `count` and `last` that panic with std's message, `x`
dropped as the panic unwinds, as `count(self)` drops what it takes,** and
the catalog names them its own, so an adapter that passes either on to its
source asks them: a `skip`'s, an `enumerate`'s, a `fuse`'s, a
`peekable`'s and a chain's second side's. What a panic says is its
message, as every panic rust-js throws is; its place, which std's
`#[track_caller]` gives as the caller's, isn't part of it. In 1.99 no
other std iterator writes such a panic: `Cycle`'s and `RepeatWith`'s
`count` and `last` are std's defaults, which run forever natively too.

Held by `test/corpus/repeat_count_panics.rs`, each caught, against native
Rust in both builds. Mutations `repeat-count-steps-forever`,
`repeat-infinite-undropped` (`runtime/repeating.ts`) and
`repeat-own-count-unknown` (`lower/recognition/own_steps.ts`);
`repeat-advance-clones` and `repeat-own-nth-unflagged` refitted.

## Amendment: stepped from their backs, a `repeat`'s field, clone and cycle (2026-11-12, #1139)

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results, and
one wrong answer fixed: what runs is std's.

**A wrong answer.** A `take(n)` of a `repeat` held in a variable is std's
struct (ADR 0871). Its `next_back` is std's `Take<Repeat<T>>::next_back`,
which inlines `Repeat::next` and reads the `Repeat`'s one field,
`element`. A `$repeating` had none, so `repeat('x').take(4)` held, then
`next_back()`, cloned `undefined` and threw where Rust gives `Some('x')`.
A `$repeating` now holds its value as std's `Repeat` does, `element`.

**Refusals lifted.** Std writes every back step of `RepeatN` and of
`Take<Repeat<T>>` as the front step (repeat_n.rs:175, take.rs:335):
`next_back` is `next`, `nth_back` is `nth`, `advance_back_by` is
`advance_by`, `rfold` and `try_rfold` are `fold` and `try_fold`, of items
each alike, and `len()` is what's left. So `$repeatingN`, and a `take` of
a `$repeating`, `$repeating(x).take(n)`, are given their back steps as
their front steps, and their length, by one runtime function,
`$backAsFront`. Their `rfold` and `try_rfold` are the runtime's default,
which steps by `back`, their `next`, so it gives what `fold` would.

The compiler's one fact of it is `recognition::steps_back_as_front`. It
also answers for a `rev()` of one, whose `next` is its source's
`next_back`, which is its `next`, held in MIR's temporary before it's
asked. It's asked where an array's `$iter` is asked, at each use, never
of what's made of one. So a map of a `repeat_n` stays `.map(f)`, and its
`collect` asks its hint, as before. The uses are `as_double_ended`,
`as_exact_size`, `has_a_back`, `len()` (`len_of_its_own`), and the
`DoubleEndedIterator` and `ExactSizeIterator` dictionaries std's compiled
`Rev` asks in the std build. These were refused:
`repeat(x).take(n).rev()` in the default build ("a `Repeat<L>` as an
`ExactSizeIterator`"), and `repeat_n(..)`'s `rev()`, `nth_back` and
`len()` in both builds.

**A `repeat`'s clone** is std's derived `Clone`, a `$repeating` of a clone
of its value. It's carried as a closure cloned apart carries its own
(`$cloning`, ADR 0522), made by `$clone`. So `repeat(x).clone()`, and
`repeat(x).cycle()`, which clones it once as `Cycle::new` does,
`$cycleOf($repeating(x), $clone, $next)`, run as Rust's do. Both were
refused, "cloning `Repeat<L>`".

Held by corpus case `repeats_stepped_from_their_backs.rs` (a value that
says when it's cloned and dropped, each back step, `len()`, a held and an
in-place `take`, a `repeat_n` of none, `rposition` of a `rev()`, `cycle()`
and `clone()`) against native Rust in both builds.
