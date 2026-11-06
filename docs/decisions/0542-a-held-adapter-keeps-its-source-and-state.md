# 0542. An adapter a struct holds keeps its source and its state

Status: Accepted. Extends [0170](0170-size-hint.md), [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md),
[0487](0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md) and [0528](0528-fuse-and-enumerate-from-either-end.md);
accepts the first part of the [iterators draft](draft/cloned-and-buffering-iterators.md)'s
core, its analysis choosing an explicit adapter before lowering.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's asked of an
adapter after it's made, its items from either end, its size hint and its
`Debug`, is Rust's.

## Context

itertools' `Positions` holds an `Enumerate<I>` and asks it, after it's
made, for its size hint, its back where its impl's bounds give `I`'s, and
its `Debug`, which shows its state, `Enumerate { iter: IntoIter([4, 5]),
count: 2 }`. Generic code's `enumerate()` was JS's `map((x, i) => [i,
x])`, which keeps neither its source nor its count: each was refused. The
draft asks one analysis to choose, before lowering, where an adapter must
keep its state, and native JS adapters elsewhere.

## Decision

**A kind of std adapter the crate's types hold is explicit wherever the
crate makes one: a JS iterator that keeps what it steps, `iter`, and its
state.** One kind is explicit everywhere in a crate, so no value of it is
converted midway.

- `analysis::held_adapters` is the one owner of which kinds: those a field
  of the crate's structs, enums or unions holds, through whatever holds
  them. `held_adapter(ty)` is the one answer the lowering asks.
- **`Enumerate` is the first kind**: `$enumerate(iter)` keeps `iter` and
  `count`, gives its own size hint, `hint`, its source's, and its back and
  length as its source's, asked as they're asked. It replaces
  `$enumerateEnds`, one helper. A held one's source is made as capable as
  its type: a range knowing its back, and an exclusive one its length.
- **A source given as it is (ADR 0411) gets its back and length where
  they're asked**: a use whose bounds give `I: DoubleEndedIterator +
  ExactSizeIterator` installs its dictionaries' on the `$iterator` the
  adapter steps, `$knowing(it, IDoubleEndedIterator.next_back,
  IExactSizeIterator.len)`, as the construction had only `I: Iterator`'s.
  `stepped_params` asks rustc what an adapter's impl asks of what it
  holds, `Enumerate<I>`'s back `I`'s length too, through a `&mut`.
  `held_knows_its_back` answers which held ones step from their back as
  they are.
- `by_ref()` of a held adapter is itself, which closing doesn't end, as a
  `$iter`'s is.
- **Its size hint is its own** (`$sizeHint` asks `hint`), and a
  `$iterator`'s that of the JS iterator it steps; a range given as it is
  still tells none, ADR 0170's difference.
- **Its `Debug` is std's**, `Enumerate { iter: .., count: n }`, of what
  its source steps, `$iteratorValue`, plain and pretty.

## Why

`held_enumerate` is `Positions`: forward, from its back by its impl's
bounds, folded from its back, its size hint and its `Debug` plain and
pretty, of a `Vec`'s iterator and of a range given as it is; a struct
holding an `Enumerate` of a range stepped from its back; an array's
`enumerate().rev()` in a crate whose types hold one; and a map of a `&mut`
to an iterator a struct holds stepped from its back, against native Rust.

## Consequences

- itertools' four `Positions` refusals and `WithPosition`'s size hint are
  gone; five remain, the draft's next kinds: `Peekable`, `Cycle`, a
  `PutBack`'s `Debug`, and a map folded from its back in a library.
- In a crate whose types hold an `Enumerate`, a chain's enumerate is
  explicit too, `$rev($enumerate($iter(indices)))`, where another crate's
  is JS's: the kind decides, not the place, so no value is converted.
- Not yet across crates: a library's held kinds aren't its consumers'.
  A consumer only steps what a library gives it, which an explicit
  adapter allows; one asking a library's adapter for its `Debug` or its
  back would need the library's kinds in its manifest.
- Mutations see no kind held, make one JS's, give a generic source no
  back, take an array's for unknowing, make a range unknowing its length,
  lend it, refuse its hint, keep no count, install nothing, read neither
  hint, and skip what an adapter's impl asks or what's behind a `&mut`.

## Amendment: a `Take`, a `Skip`, and one the crate clones

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what's given, shown
and counted is Rust's; refusals are lifted.

**An adapter that keeps a count of its own, an `Enumerate`, a `Take` or a
`Skip` (`recognition::counted_adapter`), is explicit where the crate's
types hold its kind, as before, or where the crate clones one**
(`analysis::explicit_adapters`, which `explicit_adapter(ty)`, renamed from
`held_adapter`, answers):

- `$taking(iter, n, drop)` and `$skipping(iter, n, drop)` keep what they
  step and how many they have left to take or skip, `n`, as Rust's `Take`
  and `Skip` do: forward, from the back as Rust's `next_back` steps them,
  their length, size hint and `Debug`, `Take { iter: .., n: .. }`.
  `$skipping` replaces `$skipDropping`, the skip of items with a
  destructor, which kept its count hidden: one stateful skip.
- **A clone of one is one of a clone of what it steps, at its count**:
  `$enumerate(it, numbered.count)`, where `it` is a clone of
  `$iteratorValue(numbered.iter)`, the value its type has, made as
  capable as a held one's source is. Cloning one was refused.
- Closing one closes what it steps, `$closed(iter)`, as closing an
  adapter does (ADR 0465).

A count is state a JS iterator helper hides, `map`'s index or `take`'s,
so it's the adapter's own, where a clone, a `Debug` and a back can read
it: the state ADR 0525's made-again adapters don't have.
The readability measure's `$iter` share grows, 61.6 to 61.9 in ten
thousand lines, and its renamed one, 824.7 to 825.2: a clone of an
array's iterator is a `$iter` of what it has left (ADR 0530), and the
value a clone is made of is held, `value`, `value$1`, read twice, for
its iterator and its count, in the new corpus cases that clone them.
`test/corpus/cloned_counted_adapters.rs` clones each before and after it
steps, and holds a `Take` and a `Skip` in a struct, stepped from either
end, counted and shown, against native Rust. Mutations
`cloned-counted-not-explicit`, `counted-clone-from-start`,
`counted-length-unknown`, `held-steps-enumerate-only`,
`take-debug-as-enumerate`, `enumerate-clone-count-ignored`, and
`taking.ts`'s and `skipping.ts`'s.

## Amendment: `rev()` of a `skip` or a `take` made in place

`(1..10).skip(2).rev()` and `v.iter().take(4).rev()` were refused (#117):
only a `skip` or a `take` a struct held was made knowing its back. One made
in place, of an `ExactSizeIterator`, is `$skipping` or `$taking` of its
source, made knowing its back and its length by `as_exact_size`, which
counts an inclusive range, `$rangeInclusiveLen`, and a map made in place,
`$mapEnds`, too. Of items with a destructor, which std drops as a `Vec`'s
tail slice where `take` steps past them from the back, it's refused. Held
by `test/corpus/rev_counted.rs`.

A `chain` made in place, `$lazyChain(a, b)`, is `$chainEnds` of its two
sides made knowing their backs: `a`'s then `b`'s from the front, `b`'s
then `a`'s from the back, each let go of once it's done, as std's `Chain`
does. An `Option`'s `into_iter()`, `[]` or `[x]`, steps an array as its
`iter()` does, so it's a side too. A chain held in a variable and stepped
by `next_back`, or lent to `rfind`, is still refused. Held by
`test/corpus/rev_chain.rs`.

A `step_by(n)` made in place, `src.filter((_, i) => i % n === 0)`, is
`$stepByEnds` of its source made knowing its back and its length: from the
back, the last item it keeps, found by the source's length as std's
`next_back_index` finds it. A `skip` or a `take` made in place is counted
by `as_exact_size` too, so one can be such a source. Held by
`test/corpus/rev_step_by.rs`.

## Amendment: one `Skip`, `$skipping`; a `$Steps`' `drop(n)` is it (2026-11-05, #735)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. A leak changes the
result, so Rust's behavior is kept.

The runtime had two implementations of std's `Skip`: `$skipping`, this
ADR's explicit adapter, which items with a destructor and held adapters
use, and a `$Steps`' own `drop(n)`, which the JS writes for items with
nothing to drop, as JS names a skip. rd767 made `$skipping`'s `fold` and
`try_fold` pass `unwind` to their first `nth(n - 1)` and close what they
consume as it throws. `drop(n)`'s never did. So `It(0).skip(2).fold(Acc(0),
f)` in the default build, the crate's `next` panicking within what it
skips, left `Acc(0)` and the crate's iterator undropped, where std's
`Skip::fold` (core/src/iter/adapters/skip.rs:122) drops `init`, then
`self`, as it unwinds.

`drop(n)` is now `$skipping(this, n)`, with no drop of the items, so
there's one owner of `Skip`'s steps. The JS still writes `.drop(n)`, JS's
name for it, so no output changes. `$skipping` gains its own `last`,
std's `Skip::last`: its source's `last` after its `nth(n - 1)`, which
`drop(n)` had, closed where what it skips throws or ends it, as
`last(self)` drops it. `drop(n)`'s eight mutations went with its code.

Corpus case `skip_first_nth_panics.rs` holds a `fold`, a `try_fold`, a
`count`, a `last` and a mapped `fold` whose first `nth` panics, against
native Rust in both builds. Mutations: `steps-drop-js-own`
(`runtime/steps.ts`), `skipping-last-unclosed` and
`skipping-last-unforwarded` (`runtime/skipping.ts`).

## Amendment: a held adapter's hint is `held_hint_known`'s (2026-11-06, #799)

`size_hint()` of a held adapter, an explicit one's, is found by
`held_hint_known` alone, as ADR 0561's amendment of this date says; MIR's
second list of the same, `hints_its_size`, is gone. Nothing that runs
changes (case N).

## Amendment: `Skip`'s and `Take`'s steps from the back are std's (2026-11-06, #825, #826)

`$skipping`'s `try_rfold` and `rfold` were std's only of a source with a
`tryRfold` of its own. A crate's iterator has none, as no crate writes
`try_rfold` on stable, so `skip(1).rfold(..)` and `.rfind(..)` of one
stepped its `back`, which asks its length each step, the crate's
`size_hint` too. They are now std's of any source that steps from its back:
its length asked once, then its source's `try_rfold`, its own or std's of
its `next_back` (`$tryRfoldOf`).

`$taking`, a `take(n)` made knowing its back (#718), had three gaps:

- Its `rfold` trims its source by `nth_back` first. Where that panics, in
  a stage's closure, it dropped neither the accumulator nor what was left.
  It now drops what drops with `f`, then `init`, by `unwind`, then what's
  left, as `rfold(self, init, f)` drops its parameters as it unwinds. Its
  `try_rfold`, by `&mut`, drops the first two.
- It had no `count` or `advance_by`, so it counted by stepping each item.
  std's `Take::count` is its source's `advance_by(n)`, which a reversed
  `Vec`'s `IntoIter` drops as one slice, front to back. It now has both,
  std's.
- A `take(n).rev()` of droppers made in place, collected into a `Vec`,
  was stepped by `next`, which asks `nth_back(len - n)`. std's
  `extend_trusted` folds a `TrustedLen` one, and `Take::rfold` asks
  `nth_back(len - n - 1)` and drops what that gives. `collect.rs` folded
  only std's compiled struct. It now folds one made in place too, where
  what the `nth_back` passes has a destructor (`folds_by_dropping_nth_back`),
  and asks generic code's caller `ITrustedLen` of one, as #670's rule does
  of the compiled struct. Of numbers the two orders are one, so nothing
  changes there.

ADR 0262: results change (drop order, missing drops, the crate's code run
more often), so Rust's is kept. Held by
`skip_of_own_from_its_back_asks_len_once.rs` and
`take_from_its_back_drops_as_std.rs`, both builds, and the matrix's and
conformance table's rows the nightly of afdd70519 failed; mutations
`skipping-rfold-only-of-own-try-rfold`, `taking-count-stepped`,
`taking-trim-unwind-undropped`, `take-rev-collect-stepped-not-folded`,
`generic-take-rev-collect-trust-unasked`. `$taking`'s forward steps are
now std's `Take`'s, as `$Steps.prototype.take`'s are. That is still two
`Take`s, as #735 made one `Skip`. Merging them is #830.
