# 1092. The iterator protocol is one contract: what each step owes, what it drops, and which facts it carries

Status: Accepted (the user's decision, 2026-11-05: #730, reshaped by
m6-lily's design review). This is the contract. The structural changes
that enforce it are under "The owners", each landing as its own change.
Extends
[0669](0669-an-adapter-steps-its-source-as-stds-does.md),
[0465](0465-dropping-an-adapter-is-closing-it.md),
[1089](1089-an-iterators-steps-are-passed-on-by-one-combinator.md) and
[1090](1090-an-iterator-conformance-matrix.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R throughout. What a step
drops, in which order, and which of the crate's own steps it runs are
observable, so results change wherever the JS differs, and Rust's behavior
is kept. Where this contract lets the runtime differ from std's code, it
says that no program can see the difference (case N), and why.

## Context

Of about 140 issues labelled since the taxonomy (ADR 1085), 53 are `area:
iterators`. The largest cluster is `kind: protocol gap × area: iterators`,
with 20 issues, among them #649, #705, #706, #710, #716, #719, #721, #722 and
#726. Each one was one piece of the protocol disagreeing with another:

- **the runtime's adapters:** `$Steps`, `$iterator`, `$closing`, `$holding`,
  `$given`, `$slicing`, `$forward`, `$rev`, `$flattenEnds`, `$stepBy` and
  about 30 more helpers in `src/runtime/`;
- **what generic code and std's compiled bodies are given:** the
  dictionaries (`iterator_dictionary`, `mut_ref_iterator_dictionary`), the
  compiled-call convention (`fold_entry`, `$stdIterator`, `$knowing`,
  `$took`);
- **the lowering's per-type answers:** `consumer_closes`,
  `closing_function`, `slice_seen`, `back_stepped`, `steps_may_panic`,
  `stepping_work` and the `OwnStep` catalog.

The contract was never written down whole. ADR 0465 has about 50
amendments, and ADRs 0545, 0918, 0991, 1089 and 0470 have more. Each states
one rule as one bug found it. This ADR states them all in one place, step
by step. Each rule cites std's source, `library/core/src/iter/` of the
pinned toolchain (1.99.0), abbreviated `iter/`. Each place where today's
code differs is marked **Gap Gn**.

## The contract

### 1. What a runtime iterator is

An iterator rust-js makes is a `$Steps` (`src/runtime/steps.js`, ADR
0669), or std's compiled struct made one by `$stdIterator`
(`std_iterator.js`, ADR 0561). JS's own iterators, an array's `values()`
and a `Map`'s, are given as they are where nothing below asks more of them.
`$stepsOf` makes one a `$Steps`, and `$lent` makes one a `&mut`.

Every value has exactly one **owner**, the code that drops it, as Rust's
has. The owner is one of:

- the variable or temporary that holds it, which MIR drops (`held`, §4);
- the adapter that holds it as a field, which closes it as it is closed;
- the consumer that takes it by value, which closes it at its end and as it
  unwinds (`consumer_closes`, ADR 0465's #637 amendment).

A `&mut` (`$lent`, `by_ref()`) never owns. Whatever stops early closes the
lent wrapper, not what it lends (ADR 0071).

### 2. The steps

Each step is the JS form of one std method. The JS names are camel case,
the back steps are named for `back`, and a fold's arguments come last in a
fixed order: `(init, f, [broke,] unwind, ends)`.

| JS step | Rust | Receiver | Std's default |
|---|---|---|---|
| `next()` | `next` | `&mut` | required, `iter/traits/iterator.rs:78` |
| `back()` | `next_back` | `&mut` | required, `iter/traits/double_ended.rs:97` |
| `fold(init, f, unwind, ends)` | `fold` | by value | `next` until `None`, `iterator.rs:2660` |
| `tryFold(init, f, broke, unwind, ends)` | `try_fold` | `&mut` | `next` until `None` or a break, `iterator.rs:2482` |
| `rfold(init, f, unwind, ends)` | `rfold` | by value | `next_back` until `None`, `double_ended.rs:353` |
| `tryRfold(init, f, broke, unwind, ends)` | `try_rfold` | `&mut` | `next_back` until a break, `double_ended.rs:282` |
| `nth(k, drop)` | `nth` | `&mut` | `advance_by(k)`, then `next`, `iterator.rs:388` |
| `nthBack(k, drop)` | `nth_back` | `&mut` | `advance_back_by(k)`, then `next_back`, `double_ended.rs:243` |
| `advanceBy(m, drop)` | `advance_by` | `&mut` | `try_fold` counting down, for a `Sized` iterator, `iterator.rs:328` |
| `advanceBackBy(m, drop)` | `advance_back_by` | `&mut` | `next_back` `m` times, `double_ended.rs:189` |
| `count(drop)` | `count` | by value | `fold`, `iterator.rs:224` |
| `last()` | `last` | by value | `fold(None, some)`, `iterator.rs:257` |
| `len()` | `ExactSizeIterator::len` | `&` | `size_hint`'s, asserted exact, `iter/traits/exact_size.rs:116` |
| `hint()` | `size_hint` | `&` | `(0, None)`, `iterator.rs:189` |
| `return()` | `drop` | by value | the drop glue of its fields |

`some`, `every`, `find`, `reduce` and `forEach` (JS's names, which JS code
reaching a `$Steps` calls) are std's `any`, `all`, `find`, `fold` and
`for_each`, each of the steps above. `[Symbol.iterator]()` is JS's own
protocol, which closes at its end (`closesAtEnd`).

What each step owes:

**S1. A step by `&mut` closes nothing of its receiver**, at its end or as
it unwinds: `next`, `back`, `tryFold`, `tryRfold`, `nth`, `nthBack`,
`advanceBy` and `advanceBackBy`. Its owner drops what is left. An item it
gives back is the caller's; an item it passes over is dropped as it is
passed, by `drop` (`nth`, `advanceBy`), as std's default drops each `x` in
`try_fold`'s closure (`iterator.rs:333`). A `Vec`'s `IntoIter` drops what it
passes over as one slice, front to back (`$advanceSlice`, ADR 0465's #234
amendment). That slice goes on past a panicking drop, as a slice's drop
glue does.

**S2. A step by value takes its receiver and drops it,** at its end and as
it unwinds: `fold`, `rfold`, `count` and `last`. The order of its frame's
drops is std's (ADR 0991's #710 amendment), in reverse order of the
parameters `(self, init, f)`:

- **at its end:** `f` first (`ends`, what drops with it), then what is left
  of `self`. The accumulator is the result, so it moves out.
- **as a step of `self` unwinds:** the accumulator, held by the loop's local
  `accum`, first (`unwind`), then `f`, then `self`. A step that runs before
  `init` has moved into a local, `Skip::fold`'s `nth(n - 1)` say
  (`iter/adapters/skip.rs:122`), drops `f`, then `init`, then `self`, the
  parameters' order.
- **as `f` unwinds:** `f`'s own frame has the accumulator and the item, and
  drops both. Then `f` itself, then `self`.

Two rules already written are this one:

- **`ends` (ADR 0991's #710 amendment):** an adapter's held function is
  dropped where std's fold drops it, `$holding(adapter, drop, moved)`. A fold
  by value calls `ends` before it closes what's left, at its end and as it
  unwinds (`$closedAfter`). `held_function_folded` says where each adapter's
  fold drops its function: `map`'s after the `ends` it's given, as
  `map_fold(f, g)` drops `g` first (`iter/adapters/map.rs:124`);
  `filter`'s, `filter_map`'s and `inspect`'s before it; `flat_map`'s alone,
  after its source's fold, as `FlattenCompat::fold` folds its `Map` by
  `&mut`.
- **A function std is given by value (ADR 0470's #719 amendment)** is
  dropped as the call returns and as a panic in it unwinds, since std's
  frame owns it. A function std may leave unrun is the exception (ADRs
  0469, 0490); its drop is a variant's.

**Gap G1 (#734):** the consumer's own closure is dropped after the call,
outside the fold. So every fold or `for_each` by value drops what is left
of the iterator before its closure, at its end and as it unwinds, in both
builds. Only an adapter's held function is given as `ends` (#710).
**Gap G2 (#735):** `$Steps.prototype.drop`'s `fold` and `tryFold` (a lazy
`skip()`) ran their first `nth` outside `$staged` and `$consumed`. As that
panicked they leaked `init` and never closed the source. `$skipping`, the
other `Skip`, does both. iterB's fix makes `$skipping` the only runtime
`Skip`: `drop(n)` is `$skipping(this, n)`, with std's `Skip::last`.

**S3. `unwind` is the accumulator's drop**, given by the consumer, where a
stage's panic would strand it (#423). Every implementer passes it on to
what it asks, and runs it where a step it calls throws outside `f`
(`$staged`). `ends` is what drops with `f`, an adapter's held function
(`$holding`, #710). Every fold by value passes it on, or drops it itself
before closing. `$forward` passes every argument on (`...args`), so a step
forwarded by name can't drop one (ADR 1089).
**Gap G3:** `ends` isn't a parameter of every fold. `$lazyChain`'s, `$stepBy`'s,
`$enumerate`'s, `$taking`'s and `$skipping`'s `rfold`, `$filterEnds`' `rfold`,
`$flattenEnds`' folds and `$foldBack` drop it. Their callers then drop it
after the adapter's inner sides are closed (`$closing`'s `$closedAfter`).
That is std's order only where nothing is left inside. `Chain::fold`
(`iter/adapters/chain.rs:116`) gives `f` to `b.fold`, which drops it before
`b`'s rest.

**S4. `nth` and `advanceBy` are given the items' drop**, `drop`, and give
it to every `nth` or `advanceBy` they ask (ADR 0465's #285, #289 and #492
amendments). An `ownNth` that has no drop of its own, a `take`'s past its
end, drops by `dropItem` (§4). `Take::nth` past its end drops what its
source's `nth(n - 1)` gives back (`iter/adapters/take.rs:46`). The back
steps owe the same.
**Gap G4:** two defaults of `nth_back` disagree. `$nthBack` passes `drop`
to an own `nthBack`, and `$nthBackOf` doesn't. A third copy,
`advance_back_by` written inline in `$rev`, is a fourth implementation of
`double_ended.rs:189`.

**S5. `count(drop)` and `last()`** are folds (S2). `count` drops each item
as it counts it. `last` drops each item but the last as the next one comes,
as `some(_, x)` drops its first argument (`iterator.rs:257`). As a later
drop panics, what `last` holds is leaked, as rustc leaks what a function
returns while a later drop unwinds (ADR 0465's #648 amendment).
**Gap G5:** `$Steps.prototype.last()` takes no `drop` and never drops what
it passes over. It is right only because the lowering routes every `last`
of items with a destructor to `$lastDropping`. So two owners answer
`last`'s drops, and an adapter's `last` passed on to a `$Steps` default
drops nothing.

**S6. `len()` and `hint()` read and change nothing.** Their presence is the
trait: an iterator that has `back` is a `DoubleEndedIterator`, one that has
`len` is an `ExactSizeIterator`. `$sizeHint(it)` asks `hint`, else `len`,
else std's default. A `Peekable`'s default `len` is its struct's hint
(ADR 0918's #706 amendment).

What each `size_hint` is, std's. A consumer that reserves by it, a
`collect()` into a `Vec` (ADR 0940, #728) or a `String`, `HashMap` or
`HashSet` (`string.rs:2507`, hashbrown's `extend`), shows it:

| Iterator | `size_hint()`, of its source's `(lo, hi)` (`iter/adapters/`) |
|---|---|
| a `Vec`'s, a slice's, an array's, a range's | exact, what's left |
| the crate's | its own, else `(0, None)` (`iterator.rs:189`) |
| `Map`, `Inspect`, `Enumerate`, `Rev`, `Copied`, `Cloned` | the source's (`map.rs:111`, `inspect.rs:83`, `enumerate.rs:87`, `rev.rs:56`, `copied.rs:64`) |
| `Filter`, `FilterMap`, `SkipWhile`, `MapWhile`, `Scan` | `(0, hi)` (`filter.rs:118`, `filter_map.rs:126`, `skip_while.rs:65`, `map_while.rs:49`, `scan.rs:51`) |
| `TakeWhile` | `(0, Some(0))` once its predicate failed, else `(0, hi)` (`take_while.rs:59`) |
| `Skip` | `(lo - n, hi - n)`, each saturating (`skip.rs:91`) |
| `Take` | `(0, Some(0))` at `n == 0`; else `(min(lo, n), min(hi, n))`, `hi` none as `n` (`take.rs:73`) |
| `StepBy` | before its first item, `n == 0 ? 0 : 1 + (n - 1) / step` of each bound; after, `n / step` (`step_by.rs:236`) |
| `Chain` | both sides: `lo` saturating sum, `hi` checked sum, none if either has none; one side left, that side's; none, `(0, Some(0))` (`chain.rs:182`) |
| `Zip` | `(min(lo), min(hi))`, `hi` the one either has (`zip.rs:216`) |
| `Fuse` | its source's, `(0, Some(0))` once it's done (`fuse.rs:80`) |
| `Peekable` | `(0, Some(0))` if it peeked a `None`; else its source's plus one for a peeked item, `lo` saturating and `hi` checked (`peekable.rs:76`) |
| `Flatten`, `FlatMap` | `lo` the front and back inners' `lo`s summed. Of inners of a fixed size `N` (`[T; N]`), plus `N` times the source's `lo`, and `hi` exact likewise. Else `hi` the inners' `hi`s summed only once the source's hint is `(0, Some(0))`, otherwise none (`flatten.rs:534`). Of inners that give at most one item (`Option`, `Result`), `(0, hi)`, or the source's where each gives one (`flatten.rs:864`) |
| `Cycle` | `(0, Some(0))` of an empty source, `(0, None)` of one whose `lo` is 0, else `(usize::MAX, None)` (`cycle.rs:46`) |
| `&mut I` | `I`'s (`iterator.rs:4239`) |

**Gap G10 (#733):** the runtime's adapters' `hint` isn't std's
`size_hint` in every case, and the recognizers' chains ask no hint at all.
This table is what #733 builds against.

**S7. `return()` is the drop.** It closes once: a second `return()` drops
nothing (`$state`, `$close`'s, #870's amendment). It drops the
fields in std's declaration order, each one even if an earlier one's drop
throws, as drop glue goes on unwinding:

| Adapter | Fields, in drop order | Std |
|---|---|---|
| `Map`, `Filter`, `FilterMap`, `Inspect`, `MapWhile` | `iter`, then the function | `iter/adapters/map.rs:61`, `filter.rs:21` |
| `TakeWhile`, `SkipWhile` | `iter`, `flag`, `predicate` | `take_while.rs:17`, `skip_while.rs:17` |
| `Scan` | `iter`, `f`, `state` | `scan.rs:17` |
| `Chain` | `a`, then `b` | `chain.rs:23` |
| `Zip` | `a`, then `b` | `zip.rs:13` |
| `Flatten`, `FlatMap` | `iter` (its `Map` and so its `f`, for a `FlatMap`), `frontiter`, `backiter` | `flatten.rs:357` |
| `Peekable` | `iter`, then the peeked item | `peekable.rs:17` |
| `Cycle` | `orig`, then `iter` | `cycle.rs:15` |
| `Take`, `Skip`, `Fuse`, `Rev`, `Enumerate`, `StepBy` | `iter` | each file's struct |

`$holding` closes what it steps, then its function (ADRs 0485, 0991).
`$flattenEnds` closes its source, then both inners (#705). `$lazyZip`
closes `a`, then `b`. Each goes on past a part's throw by one owner,
`$inTurn` (`$closeEach` of iterators), as do `$lazyChain`'s, `$trustedZip`'s,
`$flatMap`'s and a comparison's two (ADR 0465's #743 amendment).

Rust aborts on a panic raised while unwinding, and so does rust-js since
#847: a drop that panics during another's unwind is `$cleanup`'s, which
aborts (ADR 0660, amended). No case under this contract depends on that
abort; the conformance table panics once a case.

**S8. Done is not dropped, but a temporary is.** A source an adapter holds
is closed once it's done, from either end or by `nth` (ADR 0570), unless it
is `held`, when its owner drops it where Rust does (#206). A temporary's
end and its source's end are the same statement, so no program sees the
difference (N).

**S9. A fold by value closes each part it finishes with, itself, where
std's frame drops it.** It never counts on S8's closing at exhaustion. Std
moves each part into the step it asks and drops it as that step ends:
- `Chain::fold` consumes `a` by `a.fold(acc, &mut f)`, then `b` by
  `b.fold(acc, f)`, which drops `f` before `b`'s rest (`chain.rs:116`);
- `FlattenCompat::fold` consumes each inner by its own `fold`
  (`flatten.rs:573`);
- `Take::fold` and `StepBy::fold` end their source with themselves
  (`take.rs:276`, `step_by.rs`).

The JS fold must `return()` each part at the same point. The consume
primitive (#734) holds what it folds, `hold()`, so nothing is closed for
being done: a part a fold leaves to be closed at exhaustion is closed only
when the whole is, after `f` and after the parts std dropped first. #757
was `$chainEnds`' fold doing so.

This isn't checked statically: whether a fold closes a part depends on
its control flow. The conformance table checks it. Its folds by value of
a closure that holds a `D` (`fold holding`, `rfold holding`, `for_each
holding`) run over a `chain`, `flatten`, `flat_map`, `step_by` and `take`
of the crate's iterators with a `Drop`, which print as each part drops.

### 3. What each std adapter asks of what it holds

Each adapter's steps are std's specializations, not the defaults. Which
step of which field each one asks is the `OwnStep` catalog's
(`recognition/own_steps.rs`, `asked_of`, ADR 1089), and the runtime
adapter's `$forward` declaration must say the same:

| Adapter | Steps std specializes (`iter/adapters/`) | What they ask |
|---|---|---|
| `Map` | `fold:124`, `try_fold:115`, `rfold:161`, `try_rfold:152` | the source's same step, of each item mapped. No `nth`, `count` or `last`: those are the defaults, so a `map()`'s `nth` asks its source's `try_fold`, never its `nth` |
| `Filter` | `count:135`, `fold:161`, `try_fold:151`, back folds `:180`, `:190` | the source's fold. `count` is `map(p as usize).sum()`, the same drops as the default (N) |
| `Skip` | `next:40` and `nth:49` (both the source's `nth`), `count:71`, `last:82`, `try_fold:104`, `fold:122`, `advance_by:137`, `nth_back:200`, `try_rfold:213` | `nth(n - 1)`, then the source's same step |
| `Take` | `nth:46`, `count:60` (`advance_by`), `try_fold:91`, `fold:116` (its `try_fold`, `:276`), `advance_by:131`, back steps `:169`–`:233` | the source's `nth`, `try_fold` and `advance_by`; from its back, `nth_back` past what it won't give |
| `Fuse` | `nth:59`, `last:64`, `count:72`, `fold:98`, `try_fold:88`, back steps | the source's same step, once (`iter: Option<I>`) |
| `Peekable` | `count:47`, `nth:56`, `last:66`, `fold:107`, `try_fold:92`, `rfold:155`, `try_rfold:135` | the peeked item first (last for `rfold`), then the source's same step. The peeked item is the match's temporary, dropped after the source's step (`count`, `nth`, `last`) |
| `Enumerate` | `nth:93`, `count:101`, `fold:129`, `try_fold:106`, `advance_by:151`, back steps | the source's same step, counted |
| `Chain` | `count:87`, `fold:116`, `try_fold:99`, `advance_by:130`, `nth:148`, `last:174`, back steps | `a`'s step, then `b`'s. `nth` advances `a`, then asks `b`'s `nth`. `last` takes `a`'s `last`, then `b`'s, and drops `a`'s once `b`'s is found |
| `Rev` | `fold:79` = `rfold`, `nth:66` = `nth_back`, `advance_by:61`, `try_fold:70`, and the reverse | the source's other end. No `last` or `count`: those are the defaults, so `rev().last()` folds, not `IntoIter::last` (#722) |
| `StepBy` | `next:59` (its first `next`, then `nth(step - 1)`), `nth:69`, `fold:82`, `try_fold:73`, back steps (`nth_back`, `rfold` and `try_rfold`: its last by `next_back_index`, then `nth_back(step - 1)`) | the source's `nth` and `nth_back` |
| `Flatten`, `FlatMap` | `fold:573`, `try_fold:556`, `advance_by:589`, `count:606`, `last:617`, back steps | each inner's same step, through the source's `fold` |
| `Zip` | `nth:92`, `fold:97`, `next_back:122` (trims the longer side, dropping what it trims) | each side's `next` |
| `Copied`, `Cloned` | `Copied`: `nth:84`, `last:88`, `count:92`, `advance_by:97` and folds; `Cloned`: folds only | the source's same step |
| `TakeWhile`, `MapWhile`, `Scan` | `try_fold` only | the source's `try_fold`, stopped |
| `SkipWhile` | `try_fold:71`, `fold:87` | its first `find`, then the source's fold |
| `Inspect`, `FilterMap` | folds, and `next_back` | the source's fold |
| `Cycle` | `try_fold:56`, `advance_by:87` | its `iter`, then fresh clones of `orig` |
| `&mut I` | `next`, `size_hint`, `advance_by`, `nth`, `fold` (= `I::try_fold`), `try_fold` (`iterator.rs:4233`); `next_back`, `advance_back_by`, `nth_back`, `try_rfold` (`double_ended.rs:437`) | `I`'s. Not `count`, `last` or `rfold`, which are the defaults over these |

**Gap G6 (#736):** no runtime flatten (`$Steps.flatMap`, `$flatMap`,
`$flattenEnds`) has a `count`, and `asked_of` has no row for it. So a
`flatten()`'s `count` never calls an inner's own `count`.

**Gap G7:** most adapters are implemented two or three times, and the
implementations disagree:

| Std adapter | Runtime implementers |
|---|---|
| `Skip` | `$Steps.drop`, `$skipping` |
| `Take` | `$Steps.take`, `$taking` |
| `Map` | `$Steps.map`, `$mapEnds` |
| `Filter` | `$Steps.filter`, `$filterEnds`, `$filterDropping` |
| `Chain` | `$lazyChain` alone since #739, which knows its back where both sides do (ADR 0545's #739 amendment); `test/architecture-iterators.test.ts` refuses a second |
| `Flatten`, `FlatMap` | `$Steps.flatMap`, `$flatMap`, `$flattenEnds` |
| `StepBy` | `$stepBy`, `$stepByEnds` |
| `Zip` | `$lazyZip`, `$trustedZip` |

Std's compiled struct is one more of each (`--std-iterators`). G2 is one
such disagreement. #621 (`$mapEnds` had no `fold`) and #705 were others.

### 4. The facts a value carries

A fact is something a step's caller asks of a value beyond its steps. Each
one has one meaning and one maker:

| Fact | Meaning | Made by | Asked by |
|---|---|---|---|
| `ownFold`, `ownRfold`, `ownNth`, `ownNthBack`, `ownLast`, `ownCount` (`$ownStep(step)`) | calling this step runs one that drops what it passes, or takes and drops the value, itself: the crate's own, a `Vec`'s slice step, or a compiled body | `$iterator` (the crate's own), `$slicing` (a `Vec`'s), `$stdIterator` (compiled `nth`, `last`, `nth_back`), `$forward` for a step passed on by name, with no `each`, from a source that has the fact, and each adapter whose own `nth` or `nth_back` passes it on to its source's, from a source that has it (`$skipping`, `$taking`, `$enumerate`, `$stepBy`, `$stepByEnds`, #834) | `$skipping`, `$stepBy`, `$closing`, `$took`, `$lastDropping`, `$count` |
| `dropItem` | the items' drop that an `ownNth` with none of its own drops by | `$slicing`, `$closing`, `$skipping`, `$lazyChain`, `$rev`, `$Steps.take` | `take`'s `nth`, `$lazyChain`'s `nth` |
| `held` | its owner is a variable or a temporary (S8); passed on to `from` and `iter` | `$held`, `hold()` | every done-closing |
| `from`, `iter` | what it steps, for `hold` and for `$knowing`'s search | each adapter | `hold`, `$knowing` |
| `back`, `len`, `hint` | the traits it has (S6) | each adapter, of its source's | `$rev`, `$taking`, `$enumerate`, `$sizeHint` |
| `of` | the value `$iterator` steps: the crate's struct, a range's bounds | `$iterator`, and `$rev` passes it on (ADR 0545's #721 note) | the iterator's drop (`drop(it.of)`), clones, `std_types/range.rs`'s `count`, `last` and `nth` |
| `items`, `at`, `end`, and its own `advanceBy` and `advanceBackBy` | a `Vec`'s, an array's or a `VecDeque`'s `IntoIter`, which drops what it passes as one slice | `$iter`, `$slicing` | `$closing`, `$given`, `$advanceSlice` |
| `$steps`, `$steps.takes` | std's compiled struct, the steps that take it | `$stdIterator` | `$took`, `$lent`, `$closing` |
| `$state` | its lifecycle: open, taking, taken or closed (#870's amendment) | `$close`, `$takenBy`, `$stateFrom` | `$close`, `$takenBy` |
| `given`, `with` | a type parameter's value made a JS iterator, and another of a clone | `$given`, `$iterator` | `$givenClone` |
| `knowing(back, len, rfold, nthBack, tryRfold)` | a compiled struct given its back by a use site | `$stdIterator` | `$knowing` |
| `closesAtEnd` | stepped by JS's own protocol | `[Symbol.iterator]` | itself |

**Gap G8:** facts are read by name, with no API. `own*` is read in 8
runtime files, the slice fact as `Object.hasOwn(it, "advanceBackBy")` or
`it.items !== undefined` in 13, and `dropItem` in 6. The presence of a step
also stands for two other facts. A `$Steps` always has `fold` on its
prototype, so readers use `typeof it.fold === "function"`,
`Object.hasOwn(it, "fold")` and `it.fold !== undefined` to ask three
different things: is it a JS iterator, is the step specialized, is it the
crate's own. The lowering reads `.of` of the JS it was given, and
`std_types/range.rs` matched a `Member(_, "of")`. That was #721's cause: a
per-type answer met a value of another shape.

### 5. What generic code and std's compiled bodies are given

- **A type parameter's value** is given as it is, or as `$given(value,
  iterator, boxed, back, len)` of its dictionaries (ADR 0411). Its
  dictionaries name std's helpers that ask the value at run time (`$next`,
  `$nth`, `$last`, `$count`, `$foldOf`, `$foldBack`, `$advanceSlice`),
  `$askedStep`'s one table. Whether a dictionary's step took the value is
  `$took(step, value)` (ADR 1089's #621 amendment).
- **A `&mut I`'s dictionary** passes on what std's impl passes on (§3,
  `mut_ref_iterator_dictionary`), and is given `$lent`.
- **Std's compiled struct** is `$stdIterator(struct, steps)`, whose `steps`
  are the compiled bodies (`std_iterator_steps`, `compiled_fold`). Its
  back, length and back folds are a use site's, `$knowing(..)`. A compiled
  body that takes the struct (`steps.takes`) leaves it taken once the
  call ends, though the call throws (rd764, #649).
- **A fold entry** is `(iterator, init, f, unwind, ends)`
  (`fold_entry`), which passes `unwind` and `ends` on as the runtime's
  folds do (S3).

### 6. What the lowering decides of each value

Each of these is one owner's answer, today asked per type:

| Fact | Owner | Its JS |
|---|---|---|
| who closes a consumed iterator | `consumer_closes` (`mir/iter/closing.rs:107`) | `$closing(it, drop)` |
| whether an adapter holds a function with a destructor, and where its fold drops it | `closing_function` (`mir/iter.rs:268`), `held_function_folded` | `$holding(it, drop, moved)` |
| whether a `Vec`'s value drops what it passes as one slice where its type is lost | `slice_seen`, `slicing` (`mir/iter/closing.rs:510`) | `$slicing(it, drop)` |
| which of the crate's own steps std's adapters reach | `reaches_its_own`, `OwnStep` (`recognition/own_steps.rs`) | which helper is called |
| whether a value is stepped from its back, and so made knowing it | `back_stepped` (`analysis/back_stepped.rs`), `knowing_held` (`iterators/knowing.rs:1234`) | `$knowing(..)`, `$rev(..)` |
| whether stepping may panic | `steps_may_panic` (`mir/iter/collect.rs:394`), `stepping_work` | `unwind`, `$collectDropping` |
| what a type parameter is given | `iterator_dictionary` (`iterators/dictionaries.rs:228`) | `$given(..)` and dictionaries |
| whether a struct is compiled where it's made | `compiled_struct_at`, `in_place_adapters` | `$stdIterator(..)` or the runtime's |

One rule here is already written. **What a `flat_map()`'s closure gives is
made in place only where the `flat_map()` is** (ADR 0545's #716 and #726
amendment). A flatten made knowing its back marks what its closure gives as
made in place, a fact of its own, not one judged by the outer call's span
(#716). A held `flat_map()` is std's compiled `FlatMap`, whose closure's
result is not made in place (#726).

**Gap G9:** these answers are per type, but each is emitted as its own
wrapper around a value: `$closing`, `$holding`, `$slicing`, `$knowing`,
`$given`. So a value's facts are the composition of five wrappers, each
reading the facts the one inside it left. #721 (a per-type `back_stepped`
answer met at a `count()` that read `.of`) and #716/#726 (an in-place
answer judged by another call's span) were this gap.

## The owners

The user adopted m6-lily's design review (#730, 2026-11-05). A written
contract doesn't remove the bug class alone. It has two root causes, and
each gets one owner:

- **(a) Rust's drop order for by-value parameters is modelled nowhere in one
  place.** Each helper re-derives who drops what, and when, in its own
  `finally`. #710, #719 and #734 are all S2. **One runtime primitive
  consumes by value.** Every by-value `fold`, `for_each`, `sum`, `count`,
  `last`, `collect` and `extend` goes through it. It takes `(init, f,
  unwind, ends)` and enforces S2's order on every path. Adapters only pass
  it on. An architecture test refuses a consume that doesn't go through it.
  Owner: m6-lily, through #734. It closes G1, G2 and G3, and G5 where `last`
  is one of its consumers.
- **(b) Facts travel as side channels on JS shapes, spans and runtime
  properties:** `.of` (#721), the in-place mark and the outer span (#716),
  `results_in_place` keyed by span (#726), and `$closing`'s arrow shape.
  **One lowering-side iterator value**, `{ expr, ty, made_in_place,
  knows_back, holds_fn, bounds, … }`, carries §6's answers for each value.
  Every recognizer and consumer reads its facts from it, never from `Expr`
  shapes, spans or runtime properties. An architecture test refuses shape
  readers (`closed_parts`, `stepped_form`, `eager.rs`'s `$closing` readers)
  and span-keyed fact tables (`in_place_adapter_steps`,
  `results_in_place`). Owner: #730's proto agent, after the conformance
  table. It closes G8's lowering half and G9.
- **(c) Std's compiled adapters are the oracle, not the only
  implementation.** The `--std-iterators` build is std's own MIR, so where
  it runs, it does what §3 says by construction. The hand-written runtime,
  which reads as a person writes JS, is proved against it and against
  native Rust: the conformance table, and per push #731's differential
  slice of cases whose closures capture what has a destructor. G4, G6 and
  G7 are found by it and fixed one by one. A disagreement between the two
  builds is a wrong answer in one of them.

No large move of helpers happens before the conformance table exists.

## The conformance table

`test/iterator-matrix.ts`'s `conformanceCases()` is a second table of ADR
1090's generator. It is built from the same sources, adapters and
consumers, and run by the same harness:
`ITERATOR_TABLE=conformance bun test test/iterator-matrix.test.ts`. Its
known failures are rows of the matrix's one list,
`test/iterator-matrix-known.txt`, under the same issues: `conformance:plain`
and `conformance:std`, each with its table's generator version, taken off
by the same `take-off`.

Where the matrix pairs two adapters over a few consumers, this table takes
each adapter alone, and no adapter at all, and runs it through every step
of §2 that it has:

- the matrix's consumers;
- `next` with the rest dropped, `next_back`, `nth_back`, `rfind`
  (`try_rfold`), `find`, `position` and `for_each`;
- `size_hint`, `len`, and the `capacity` of a collect into a `Vec`.

Each case runs:

- plain;
- with an item's drop panicking;
- with the adapter's closure panicking, and with the consumer's;
- with closures that hold a `D` of their own (`map holding`, and `fold
  holding`, `rfold holding`, `for_each holding`, `try_fold holding` and
  `any holding`), so S2's order shows (#734).

Each runs natively and in both builds. The `--std-iterators` build is the
second oracle, as (c) says. The adapters are the matrix's and those it
leaves out: `take_while`, `skip_while`, `map_while`, `scan`, `inspect`,
`filter_map`, `flat_map` and `by_ref`. They run over the sources that tell
their steps apart:

- a `Vec`'s, a reversed `Vec`'s, a range and a slice's;
- the crate's iterators that write none of their steps (`Plain`), a
  `size_hint` (`Hints`, `Exact`, which print as it runs, #733), every step
  (`Every`), or a back (`Backs`);
- a type parameter of a `Vec`'s and of the crate's.

That is 21,594 cases in 360 programs. Its known failures are listed as the
matrix's are, under their issues, and only shrink.

## Alternatives

- **Run std's compiled adapters everywhere,** making `--std-iterators` the
  only build. Std's own bodies would be the one implementation of §3.
  Rejected as the only implementation and adopted as the oracle (c):
  - #734 and #736 are wrong in that build too, so S2 and §4 still need
    owners;
  - the plain build's output is the product: `v.iter().map(f).filter(g)` is
    JS a person writes, and std's compiled bodies and dictionaries are not;
  - the crate's own iterators, JS's, and the dictionaries still need the
    base protocol.
- **A single runtime owner, `protocol.js`, with one constructor of every
  adapter and one implementation of each,** proposed by this ADR's first
  draft. It would fix G4, G5 and G7 in one move. Deferred: the order the
  user chose fixes the two root causes first, (a) and (b), and lets the
  oracle (c) find the rest. The runtime half of G8, facts read by property
  name, remains its argument, to be weighed once (a) lands.
- **A class per adapter, overriding `$Steps`' methods.** Rejected, as in ADR
  1089: the adapters close over their state, and `$copy` and `$become` copy
  fields.
- **A catalog generated from std's source**, which step each adapter's impl
  asks, read from `library/core`. Rejected: the bodies aren't regular enough
  to read reliably. The conformance table checks the catalog against std's
  behavior instead.
- **The contract and the table alone.** Rejected: the 20 issues came from
  places drifting apart, and a table finds drift only after it has
  happened.

## Consequences

- This ADR is the contract a change to an iterator step is checked
  against. Its gaps G1–G10 are #730's work list:
  - G1 is #734, G2 is #735 and G6 is #736, each P0, filed from this
    contract's probes;
  - G10 is #733.
- The conformance table's first runs found these P0s, each minimized
  against native Rust:
  - #738: `rfind` leaks the item its predicate panicked on (S1);
  - #739: `$chainEnds` lacks a chain's front steps (G7);
  - #743: a chain's and a zip's drop stop at their first side's panic
    (S7);
  - #744: `$flattenEnds`' folds don't pass `unwind` to their closure
    (S3);
  - #745: `len` and `size_hint` are read off an assumed JS shape (G9);
  - #746: a compiled `scan()`'s collect drops in the wrong order;
  - #747: a `&mut I`'s `rfold` reaches `I::rfold` (§3);
  - #748: a compiled `scan()` consumed by value drops its source twice;
  - #749: a type parameter's `by_ref()` is the value itself (§3);
  - #762: a `map()`'s held function is leaked past its source's panic
    (S7), and dropped after what's left from its back (S2).

  Their cases are listed under each in `test/iterator-matrix-known.txt`,
  which only shrinks:
  - 953 under #734 are the folds by value of a closure that holds a `D`
    that #734's first step doesn't reach yet: generic code, `rfold`, the
    `*_while`s and `scan`, and std's build;
  - #744's were fixed before they were listed.

  A collected `Vec`'s capacity number is ADR 0419's case B: the table
  checks only `capacity() >= len()`. A closure made by a block given to the
  call is refused (#759, P2), and a few of those cases stay in the table as
  refusals.
- docs/architecture.md's "Libraries and runtime" names the contract and its
  owners.
- The stale references to "ADR 1092" for the iterator matrix
  (`test/iterator-matrix.ts`, ADR 0510) now name 1090, its number.

## Amendment: the lowering's iterator value, step by step (2026-11-05, #730 phase 3)

Owner (b) lands as `src/lower/iterators/value.rs`'s `IterValue`, in nine
steps, each its own commit. The user decided two things: the value sits
beside each MIR local's `Expr`, and its facts are kept per local only. A
struct's field, a closure's capture and a return fall back to the type's
answer.

Many of the readers it replaces run in tidy's passes, after lowering
(`range_items`, `reversed_step`, `eager_chains`, `reversed_in_place`, tidy's
loops). No MIR local exists there. So the value's facts travel on the JS it
writes, as `Known::Part` carries a tuple's part (#756):

- `IterValue::emit()` is the one writer of `$iterator`, `$given`,
  `$knowing`, `$closing` and `$lent`.
- `emit()` marks each call with what it is: `Known::IterStepped`,
  `IterRange`, `IterKnowing`, `IterClosing` and `IterLent`.
- A reader asks value.rs, which reads those facts, never a helper's name or
  a call's shape.
- Value.rs reads the shape back itself only where no fact can reach the
  reader: after a spill to a variable, or where tidy builds the shape
  itself. Each such site is allow-listed, with its reason, in
  test/architecture-iterators.test.ts.

Step 1 adds the value, and its makers write through it: `stepped_ends`,
`stepped_range_by`, `given_js_iterator`, `knowing_held`, `closed_by` and
`lent_iterator`. No reader changes yet, and the JS is byte-identical.
`$holding`'s maker, `closing_function`, stays m6-lily's.

Step 2 reads a range's bounds through the value. `stepped_range` and
`stepped_from` move to value.rs and answer only of a call `emit()` marked
`Known::IterRange`. A range's bounds read, `it.of`, is value.rs's
`bounds_of`, and tidy's `reversed_step` folds it through `bounds_read`, never
by matching a member named `of`. Tidy's rewrites that make a range stepped
from its end, `reversed_inward`'s and `reversed_step`'s `.drop(n)`, make it
through `stepped_range_by`, so what they make is marked as the lowering's is.
A value made again of the same call keeps what its maker marked it
(value.rs's `its_facts`): `iterated`, `from_iterator` and tidy's `unarrayed`
rebuild the call they're given without what else was known of it, and had
dropped the marks with it. Then a `for` over `(a..b).rev()` was no longer
the `for` that counts down.

Step 3 counts what a value made in place has left through the value.
`len_in_place` reads what a range's or the crate's value steps by value.rs's
`steps`, which answers only of a call `emit()` marked
`Known::IterStepped`. #745's `NaN`, a held `Rev` counted as its range, was
fixed first on its own (d9dfd1cff), so this step changes no JS.



## Amendment: a first step taken while a fold's frame holds only its parameters drops `f` before the accumulator (#768)

S2's step that runs before `init` has moved into a local covers three
adapters, in both `fold` and `try_fold`:

- `SkipWhile::fold(mut self, mut init, mut fold)` and `try_fold(&mut self,
  mut init, mut fold)` (`iter/adapters/skip_while.rs`), with their first
  step, the predicate's `find`;
- `Skip::fold` and `try_fold`, with their `nth(n - 1)`;
- `StepBy`'s `spec_fold` and `spec_try_fold`, with their first `next()`.

Each takes its step while its frame holds only its parameters. A panic
there drops `fold` first, then `init`, then (of `fold`) `self`. Past that
step, the source's own fold holds the accumulator in a local, which drops
first. In both builds the JS dropped the accumulator first. The consumer's
closure was dropped after the call, outside the fold, and std's compiled
`try_fold` was given no drop of its `F`.

- **`tryFold` and `tryRfold` take `ends` too**, `(init, f, broke, unwind,
  ends)`, as the table above now says. Every runtime `tryFold` and
  `tryRfold` takes it and passes it on where std moves `f` into its
  source's: a map's, a filter's, a take's, an enumerate's and a scan's.
  `$flatMap`'s and `$Steps`' `flatMap`'s don't pass it on, as `FlattenCompat` folds
  its source by a closure that borrows `fold`. `$Steps`' default leaves it
  to its caller.
  - An architecture test holds this, as it holds fold's (#734). The chain's
    are left to iterD's merged `Chain` (`$lazyChain`, `$chainEnds`), which
    leave that test's list once they take it.
- **The owners.**
  - `$tryFoldSome`, `$tryFoldOk` and `$tryFoldInto` give `ends` through
    `$endsAfter(ends, run)`. It drops `ends` once, at the call's end and
    as it unwinds, as `try_fold` drops `f` then, or earlier where an
    adapter runs it first (`$onceOf`).
  - The first step is `$stepHolding(step, init, unwind, ends)`'s. It runs
    `ends`, then `unwind`, the second though the first throws. It's used
    by `$lazySkipWhile`, `$skipping` and `$stepBy`.
  - `$skipping`'s `fold` and `try_fold` are std's of any source with an
    `nth`, not only one with its own.
  - `$stdIterator` passes `ends` on to a compiled `try_fold` and
    `try_rfold`, whose `F`'s drop it is (`compiled_try_fold`), as a
    compiled `fold`'s `ends` is. A compiled body gives its own `try_fold`
    the `F` it was given.
- **Which consumer's closure goes with its fold is `consumed_function`'s
  one answer**, for `ends` and for the call's own drop after it
  (`dropped_given`). It now also answers yes for `fold`, `rfold`,
  `try_fold` and `try_rfold` when two things hold: the closure drops, and
  its accumulator has a destructor that a stage's panic may strand
  (`stranded_drop`'s condition). Only then is the order of the two drops
  observable, and only the fold knows it.
- **The default build's in-place `step_by()` (#773).** It was a filter by
  index, whose first step is inside its source's fold, so it never
  reached `$stepBy`. Now `IterComb::StepBy` makes it `$stepBy` wherever
  its source's steps may panic (`steps_may_panic`): the adapter is made
  before its consumer is known, so it can't ask whether a fold strands an
  accumulator. Of the corpus, `panic_mid_fold_drops.js`'s collect of a
  `map(..).step_by(2)` changes form so. It's 4 lines fewer, which moves
  the readability gate's like-for-like `$iter` and `renamed x$1` shares
  up by 0.1 and 0.2 in 10,000 lines, blessed for this. Case: results
  change.
- Case: results change (ADR 0262). Held by
  `test/corpus/skip_while_first_step_unwinds.rs` in both builds, and by the
  conformance table's `skip_while | fold holding` and `skip_while |
  try_fold holding` cases at `stage`, taken off its list.

Step 4 reads whether a value is closed or lent through the value. Value.rs's
`closed` (`it`, `drop`, `passes_each`), `closed_parts`, `unlent` and `is_lent`
answer only of a call `emit()` marked `Known::IterClosing` or `IterLent`.
They replace iterators.rs's `closed_parts` and in_place.rs's `unlent` and
`is_lent`, and eager.rs's and `reversed_inward`'s reads of `$closing` by its
name. The readers are knowing.rs's remakes of a closed or lent value
(#223, #225), `slicing`, a trusted-length `eq`, `positioned`, `range_items`,
eager chains and `found_index`.

Step 5 reads what a value steps through the value for the rest of its
readers. `stepped_form` goes: its readers ask value.rs's `steps`.
`stepped_value`, `steps` of what isn't made in place, moves to value.rs.
Its readers are knowing.rs's `as_exact_size` and `as_double_ended`, and
eager.rs's `consumed_into_iters`.

Step 6 gives `in_place_adapter_steps` one reader, mir/iter.rs's
`steps_in_place(span)`. `marked_in_place` makes each value's mark of it, and
`steps_adapter_in_place` and `compiled_struct_at` ask it. range.rs's
`range_own_step` asks the receiver's mark instead. The answer is still keyed
by the call's span. The span would be gone only if each in-place adapter's
maker marked what it makes and the mark carried to its sources. A source
inside a `rev()`, `v.iter().map(f).rev()`'s `map()`, has no mark of its own
today, so that is a change in what's decided, not a mechanical one. It is
#730's later step, after iterC's knowing.rs work, proved against the corpus
case by case.

Step 7 makes what a `flat_map()`'s closure gives a fact of the closure's
value. `body_queries::results_in_place`, the THIR owner, still decides it
before lowering. The closure's lowering has to know it, and the closure is
made before the `flat_map()` call it's given to. Its answer is now also
`BodyFacts::result_in_place`. `mir_closure` marks that closure's arrow
`Known::IterResultInPlace`, and `iterable_of_each` and `closed_inner` keep
the mark on the arrows they make of it. knowing.rs's flatten branch asks the
mark (`gives_made_in_place`) where it judged `made.is_some()` of the
flatten's type (#716). A held `flat_map()`'s closure, std's compiled
`FlatMap`'s, is never marked (#726). So step 9 doesn't refuse
`results_in_place` itself, as the design first said: it is the THIR owner's
question, and only its answer's carrier changed.

Step 9 holds what steps 1 to 7 made true, in
test/architecture-iterators.test.ts. Each failure names value.rs as the
owner, where a new fact goes. Outside value.rs, the test refuses:

- a definition of an iterator value's readers;
- a write of `$iterator`, `$given`, `$knowing`, `$closing` or `$lent`;
- an iterator fact read off a member's name (`of`, `at`, `end`, `iter`,
  `items`);
- a read of `in_place_adapter_steps` but by body_queries, its owner,
  `--std-iterators`' switch, and `steps_in_place`.

The reads of an iterator helper by its name that remain are listed per file
and helper, each with its reason and what removes it. A new one fails, and
so does a gone one until its entry goes. They are the `$rev`, `$stepBy` and
`$repeatingN` readers, whose makers aren't in value.rs yet (step 10), and the
`$iterator`/`$knowing` readers of `OwnMethods` and tidy's `stepped_in_place`.

Step 10 moves the makers of `$rev`, `$stepBy` and `$repeatingN` into
value.rs: `reversed_by`, `stepped_every` and `repeating_n`. Each marks what
it writes (`Known::IterRev`, `IterStepBy`, `IterRepeatingN`), one of the
kept marks. Their readers ask `reversed_of`, `is_reversed`,
`stepped_every_parts` and `repeating_n_parts`, never the helper's name:
`reversal`, `stepped_by`, tidy's `stepped_in_place`, `reversed_in_place`,
`range_step` and `reversed_step`, `len_in_place` and `as_double_ended`. Step
9's test now refuses a write of any of the three outside value.rs. The
named reads left are the `$iterator` readers of an own iterator's steps and
of a range stepped from its end, and tidy's `$knowing` reader, each listed
with the accessor value.rs doesn't give yet.

Step 11 gives value.rs the accessors the last named reads lacked.
- `emit()` marks an `$iterator` of the crate's own or a dictionary's steps
  `Known::IterOwn`.
- `iterator_parts` gives an `$iterator`'s arguments, not `$given`'s. It is
  read by `behind_a_dyn`, by tidy's `stepped_in_place` dropping a hint, and
  by `reversal`'s range stepped from its end.
- `is_range_stepped` moves to value.rs.
- `is_knowing` reads `IterKnowing`, for tidy's `stepped_in_place`.

Step 9's list of named reads is now empty. No iterator helper is read by its
name outside value.rs.

Tidy's `stepped_in_place` reads `$knowing($iter(xs))` as `$iter(xs)`. Since
step 1, `emit()` is the one writer of `$knowing`, and its one maker,
`knowing_held`, writes it only of std's compiled struct, never of an array's
iterator. Tidy makes that shape itself, though: stepping what `$knowing` is
given in place, a `fuse()` given no drops is its source, and that is an
array's iterator. `fused_iterator.rs` is that case. Its two halves are held
by `knowing-unread` and `knowing-of-items-kept`.

Step 8 holds who reads back_stepped's per-type set, whether a value is made
knowing its back (ADR 0545). Its readers are the makers (`cloned_apart_call`,
`mir_iter_std_call`, `back_kept`, `rev_called`, `reversed_knowing`) and two
type questions asked with no value in hand (`knows_its_back`,
`knows_its_length`). test/architecture-iterators.test.ts lists them per file,
with their counts and reasons. One value is read past its maker:
`as_double_ended`'s `rev()` made in place. A mark the maker gave it would be
lost where MIR spills it to a variable, which the type's answer isn't. So
that read stays listed until step 8b. Step 8b carries each local's marks
across a spill: recorded where it's written to a variable (`flush`,
`mir_store`), given back where it's read (`read_local_value`). Step 6b needs
the same, and they come together.

Step 8b's first part carries each MIR local's value marks across a spill to
its variable, `temporaries::IterLocals::marks`, the value beside each local
as the user decided. It records them at `flush`'s `const` and at
`mir_store`'s `const` or `let` of a local's value, and gives them back where
`read_local_value` reads the variable by name. It doesn't cover yet:
- a `let` assigned after;
- a read through a ref or a place (`refs`, `places`), `it.rev().rfind(..)`'s
  `&mut` receiver;
- a value `spill` names, other than `spill_marked`'s.
No reader asks a variable's marks yet. `as_double_ended` asking a `rev()`'s
`IterKnowsBack` (step 8b's second part) waits on those paths: with only
these two, `rev_made_knowing_its_back.rs` and
`own_iterator_flattened_from_its_back.rs` lost the mark and were refused.

Step 8b's second part finishes the carrier and moves that read onto the mark.
- **Recorded at every whole write** of a local's variable (`note_marks`):
  `flush`'s `const`, and `mir_store`'s `const`, `let` and assignment, the
  `let` a body declares first for its cleanup and assigns after.
- **Only of a local MIR writes once**, by an assignment or as a call's
  destination (`IterLocals::once`). One written again keeps none, since a
  read may be of either value, a loop's. A whole `&mut` to it,
  `it.rev().rfind(..)`'s receiver, isn't a write here: a value made in
  place is given only to what steps it (`in_place_adapters`), which never
  writes it whole; one given elsewhere is std's struct, which no maker of
  a mark makes.
- **Given back on every read by name** (`read_by_name`): `read_local_value`'s
  and a place's base (`place_expr`), which the `&mut` receiver is read
  through. A value read in place, through a ref or a place (`in_place`,
  `refs`, `places`), is the value's own JS and keeps its marks already.
- **Kept across the two spills of an iterator**: `$held(..)`, the iterator
  its owner drops (`held_by_its_owner`, #206), keeps `IterKnowsBack`, as it
  is the same iterator; `spill_marked`'s `iter` keeps all of `its_facts`.

`reversed_knowing` marks both its results `Known::IterKnowsBack`
(`made_knowing_its_back`), and `as_double_ended` asks the value
(`was_made_knowing_its_back`), not the type's set. The mark is given only
where the type's set says so, so the read is never looser than the type's
answer. A value of that type a field, a capture or a return gives has no
mark, but it isn't made in place either: it is std's struct (ADR 0871),
which the read never took. So the read's second question, `made_in_place`,
goes: the recognizers' `rev()` is the mark's one maker, and it makes none
std's struct, which a field, a compiled body or a `let` read past
`Iterator`'s methods holds (`in_place_adapters`). The whole
corpus and the corpus with std's iterator structs pass without it, so
`rev-made-knowing-of-a-held-struct`, which removed it, goes too. The corpus,
both slices and the corpus with std's iterator structs are byte-identical.
back_stepped's set has no read of a value past its maker left, and
knowing.rs's row of step 8's list drops it.

Step 6b moves the in-place answer from the span of the call that steps an
adapter to a mark its maker gives the value. `in_place_adapter_steps` was a
set of spans, each of a call where some argument is an adapter made in place.
`compiled_struct_at(ty, span)` asked that set of whatever type it was given,
so a source's type, or the type of a value the call didn't step, got the
call's answer.
- **The maker marks.** A call's value is marked `Known::InPlace` where
  `adapter_in_place` says the call makes an adapter in place (`made_by` in
  mir/iter.rs, which the call's lowering applies to what it gives). A `rev()` is marked before `reversed_knowing` makes it
  knowing its back, which asks it (`rev_called`). What `rev_items` reverses,
  made again to step from its back, keeps its source's mark. So does a
  `skip(n)`, `take(n)` or `step_by(n)` a temporary holds for a back step,
  made knowing its back there (`made_knowing_where_held`, #718). Without
  that, its back step called std's compiled `Skip`'s steps, and a range's
  `step_by(2)` stepped by `nth_back` then `fold` panicked: the iterator
  matrix's slice and `skip_and_take_stepped_from_their_back_in_place.rs`
  caught it before the readers moved. The recognizers' `take(n)` and
  `skip(n)` marked themselves in place too (#718). With `made_by` that mark
  is the maker's, so their own go, and with them their mutations,
  `take-made-unmarked-in-place` and `skip-made-unmarked-in-place`, which
  survived. The whole corpus, both slices and the std-structs corpus pass
  without it.
- **The mark is carried** wherever the value is held. `held_marks` is
  `its_facts` and `InPlace`: a local keeps it (`note_marks`), a spill's name
  keeps it (`spill`, so `spill_marked` goes), and `$held(..)` keeps it.
- **The readers ask the value.** `compiled_struct(ty, &value)` replaces
  `compiled_struct_at(ty, span)` at its 15 callers. A collect asks what it
  was given before its fold. Dispatch's `steps_adapter_in_place` asks the
  receiver's mark.
- **What goes.** `marked_in_place`, which marked a call's arguments by its
  span, and `steps_in_place` and the span set. body_queries' fixed point
  keeps only the adapters made in place. test/architecture-iterators.test.ts
  refuses any of these names coming back.

ADR 0262's case: none, as the output is unchanged; the decision moved to its
owner. Before the readers asked the mark, the corpus showed two ways a mark
was lost, and each was fixed:
- a spill named a value without its mark (`collected_counts_ask_room.rs`,
  `range_steps_anywhere.rs`, `range_steps_known.rs`);
- a `rev()`'s source was made again without it (`lengths_and_backs_given_on.rs`,
  `own_iterator_flattened_from_its_back.rs`).

With every reader on the mark, the span and the mark still disagree at 110
`knowing_held` calls in 31 cases, all one way. The value is a `zip()`,
`chain()`, `flatten()`, `flat_map()`, `inspect()`, `map()` or `enumerate()`
made in place, and the span is its own maker's call. That span isn't a call
that steps it, so the old answer called it std's struct. `knowing_held`'s
own question, `!made_in_place(&it)`, already said no to it, so no answer
changed. That question was there to correct the span. Asked of the mark,
`compiled_struct` answers it already, so it goes. Its mutation,
`in-place-adapter-made-knowing`, survived the whole corpus and the corpus
with std's iterator structs, and goes too. `in-place-rev-a-struct` now holds
the rule, in `compiled_struct`. The JS is byte-identical: the whole corpus, both slices and the
corpus with std's iterator structs passed with nothing blessed.

## Amendment: each adapter's `sizeHint`, and who asks it (2026-11-05, #733 steps 1–3)

G10 closes in three steps. ADR 0262's case each time: results change, so
Rust's behavior is kept.

**The runtime's adapters each have std's `sizeHint`**, beside their other
steps, of the table above. Shared arithmetic sits in size_hint.js, one
helper each, which the adapters call: `$upperOnly` (`Filter`, `FilterMap`,
`SkipWhile`, `TakeWhile`, `MapWhile`, `Scan`), `$takeHint` (`$Steps.take`
and `$taking`, none asked of its source at `n == 0`), `$hintSum`
(`$lazyChain`, a gone side not asked), and `$flatHint` (`$Steps.flatMap`
and `$flatMap`: front, then back, then the source, fused, no upper bound
while the source may give more). `$closing` passes on what it steps', as
dropping adds no step, and `$folded` what it folds', before it's folded. A flatten's inner `Vec` is stepped as `$iter`
(`$innerOf`), so what it has left is known, as a `Vec`'s `IntoIter` knows.

**`size_hint()` reaches `$sizeHint`** of std's adapters of what's a `$Steps`
with std's hint: the crate's own, a range, an explicit adapter, a compiled
struct, a type parameter. `knowing.rs`'s `adapted_hint_known` is that one
answer, over `recognition::adapted_iterators`. Still refused:

- a chain of an array's items, whose adapters are JS's own (the
  coordinator's choice B);
- a string's `chars()` and what it makes, whose hint is of its bytes,
  `(bytes.div_ceil(4), Some(bytes))`, where its JS counted the `char`s and
  gave a wrong answer.

A flatten asked its hint is std's struct, compiled where it's made, whose
hint of `[T; N]` is `N` times its source's.

A `$iterator`'s `size_hint` argument is left off only down from a consumer
that asks no hint (a `for`, `Array.from`, `reduce`, `forEach`, `some`,
`every`, `find`, the helpers that step all), through the JS adapters
(`stepped_through`). Before, any `take`, `drop`, `filter` or `flatMap`
stripped it, so `own.take(3).size_hint()` lost the crate's hint.

**What asks the hint asks it where std does**, wherever it may run what
the program sees: `knowing.rs`'s `hint_reaches_own(ty)`, of the crate's own
`size_hint`, a type parameter's, a `dyn`'s, or what std's adapters ask of
those. Elsewhere the hint is std's own and changes only room no JS string
or table reaches.

- A collect into a `String` of `char`s, or into a `HashMap` or `HashSet`,
  is given `$reserveHint(it, size, align)`. It asks the hint once, before
  stepping, and reserves room for its lower bound, as `Extend<char>`
  (string.rs:2507) and hashbrown's `extend` do. A `BTreeMap`'s is its
  `Vec`'s.
- A `filter()`'s `count()` asks its source's hint first, std's
  `Filter::count` (filter.rs:135), as `$Steps.filter`'s own `count`.

Generic code's collects ask none yet: its JS adapters of what it's given
may be JS's own, of an array's iterator, which ask no hint, so the caller's
hint isn't reached. Their cases stay listed under #733 until the caller's
dictionary gives the hint through `$given`, the next step.

A collect into a `Vec` asks it as it grows (`$roomForHint`) only of std's
compiled structs still. Giving it to the recognizers' chains, step 4,
changes the default build's output and is the user's decision.

## Amendment: a `Vec` collect asks the crate's hint as it grows (2026-11-05, #733 step 4)

ADR 0262's case: results change, so Rust's behavior is kept. A collect
into a `Vec` is given `$roomForHint` wherever `hint_reaches_own(iter_ty)`
holds, as well as of std's compiled structs: the crate's iterator that
writes its own `size_hint`, or std's adapters of one, made in place. It
asks the hint after the first item and again each time the `Vec` is full,
std's `SpecFromIterNested` then `extend_desugared` (ADR 0940's #728
model). None of those is `TrustedLen`, which no crate iterator can be, so
it's given `false`. Elsewhere the hint is std's own, and asking it changes
only room, so ordinary collects of ranges, slices and `Vec`s print as
before. The coordinator approved the form: 11 corpus snapshots, each of a
crate iterator with its own `size_hint`.

A flatten's front inner hint (`$flatHint`) and its inner `Vec` stepped as
`$iter` (`$innerOf`) are now observable, by when a `flat_map()` collected
into a `Vec` asks its source's hint again, and their mutations hold them.

`$mapEnds` is the `map()` made knowing its back, used where its source is
stepped from its back elsewhere in the crate. It had no `sizeHint`, so a
collect of one into a `String` or a hash table asked none of the crate's
hint, where std's `Map` passes its source's on (map.rs:111). It has one
now. Twelve conformance cases (`own Every | chain | string`, `hash set`,
`hash map`) failed this way in their batch but passed alone, so they
weren't listed.

`$filterEnds` had the same gap: no `sizeHint`, and a `count` that asked no
hint. It now has std's `Filter::size_hint`, `(0, hi)` of its source's,
`$upperOnly`'s, and std's `Filter::count`, which asks its source's hint
first (filter.rs:118, 135), as `$Steps.filter` does.

## Amendment: generic code asks its caller's hint (2026-11-05, #733, the user's decision)

ADR 0262's case: results change, so Rust's behavior is kept. Generic
code's collect of a type parameter's iterator, or of std's adapters of
one, asks its hint wherever std's does: `hint_reaches_own` answers yes
for a type only its caller knows. Where the caller's value is the crate's
own iterator, `$given` passes on its dictionary's `sizeHint`, and the
caller's `size_hint` runs. Where it's JS's own, `$sizeHint` answers
`[0, undefined]`, and nothing runs.

The user chose this, (a), over a caller fact `IHint` saying whether the
hint can reach the crate's own, (b). A generic function is one JS function
for every caller, so (b) keeps every wrapper (a) adds, as a conditional,
and adds a fact to each function and an argument to each call: 71
wrappers in 62 functions, and 143 call sites in the corpus. Under (a), 42
corpus snapshots change, and neither the React port nor std/ changes.

Still left: a `Vec` collect of a type parameter's items is given room by
an item size of 0, as their layout isn't known here, so it isn't asked
again as it grows. That needs `TSize`, the fact #672's `IInPlace` already
gives.

## Amendment: what's stepped through is never the crate's hint-asked iterator (2026-11-06, #799)

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

Since a collect of the crate's iterator asks its hint through
`$roomForHint`, and `$sizeHint(..)` asks it apart, nothing that steps a
chain through, `stepped_through`, is given one whose hint is asked. Two
of its exceptions changed nothing in any of the corpus's 1,740 cases or
the 1,325 tests of the lowering, compiler, docs, library and snapshot
files, each probed where its answer would differ: a `map` of the crate's
stepped iterator left as it was, and a `$stepBy`'s source stepped through.
Both are gone, with their mutations, `map-source-hint-trimmed` and
`stepped-through-step-by-not`, and `ADAPTING_HELPERS`.

## Amendment: generic code's `Vec` grows by its caller's item size (2026-11-06, #796)

ADR 0262's case: results change, so Rust's behavior is kept. Generic
code's collect into what std grows as a `Vec` (`grows_as_vec`: a `Vec`,
a `VecDeque`, a `Box<[T]>`, or the `Vec` that a `BinaryHeap` or a
B-tree is made of, #797) now grows by its items' size, which only its
caller knows. The example is `fn all<T, I: Iterator<Item = T>>(it: I) ->
Vec<T> { it.collect() }`. Before, `$roomForHint(.., 0, false)` took those
items to have no size, so the `Vec` was never full and the caller's own
`size_hint` ran once. Native Rust asks it at 1, 5, 9 and 17.

- The size of a type parameter's or an associated type's items is its
  `TSize`, the ADR 0145 fact.
- The size of a type built from them, `(T, u8)` or `Option<I::Item>`, is
  its built slot's (ADR 0145, #737).
- A caller that knows the type gives the size, and a generic caller passes
  on what it was given.
- A trait's default copied into an impl reads it in the impl's terms.

There is one predicate for each question, so the analysis and the
lowering can't drift apart. `Recognition::hint_reaches_own`, moved from
`knowing.rs` to `recognition/lengths.rs`, says whether the hint may run
the crate's own `size_hint`. `Recognition::grows_as_vec` says whether
the output grows as a `Vec`. `type_facts`' `grown_asked` asks the size
where both hold of a collect whose items' size is a caller's. The
lowering's `grown_item_size` reads it where `$roomForHint` is given.
Where the hint is std's own, nothing the program can see changes, so the
size stays `room_item_size`'s count-alone 0 (ADR 0774) and no fact is
asked.

`adapted_iterators`, which `hint_reaches_own` follows through std's
adapters, used to take any type argument rustc couldn't rule out as an
iterator for one the adapter steps. That included a `map(f)`'s generic
`F: Fn(&T) -> U` and a `successors()`' item `T`. Now it takes only the
arguments that the adapter's own `Iterator` impl bounds by `Iterator`,
by a trait that extends it, or by a flat map's `IntoIterator`. Asking the
item size would otherwise make `pub fn applied<T, U, F>(..) -> Vec<U>`
take `U`'s size, which a JS caller can't give, so it would go undeclared
(ADR 1077). It also drops two `$roomForHint` wrappers whose hint was
std's own.

## Amendment: a `step_by(n)` from its back asks its source's `nth_back` as std's does (2026-11-06, #834)

ADR 0262's case: results change, so Rust's behavior is kept. 731ae66fe
(#823, #827) let `$stepBy`'s back step ask its source's `nth_back` only
where the source says it drops what it passes (`ownNthBack`). Only a
`Vec`'s closing and the crate's own `$iterator` said so. That made 34
rows of the matrix and the conformance table compile, and they then ran
otherwise than std's `StepBy` (found by nightly2's full run):

- **A fold from its back asked the length at each step.** `$stepByEnds`
  had no `rfold` or `try_rfold`, so `$foldBack` stepped its `back`, and
  each `back` asked `next_back_index`, the crate's own `size_hint`. Std's
  `spec_rfold` and `spec_try_rfold` take its last once, then each its
  source's `nth_back(step - 1)`. `$stepByEnds` now has both, as
  `$stepBy`'s `tryFold` and `fold` are made: the first step taken while
  the frame holds only its parameters (#768), then closed as `rfold(self)`
  drops it.
- **The crate's own `nth_back` was never called under an adapter.**
  `$enumerate`, `$skipping` and `$taking` pass `nth_back` on to their
  source's, and `$stepByEnds`' is its source's past the steps, but none
  carried `ownNthBack` as they carry `ownNth`. Std's compiled struct, a
  `fuse()`'s, was given its compiled `nth_back` by `$knowing`, but not
  marked as its compiled `nth` is. Each now carries the fact from a source
  that has it, and the compiled struct is marked. So
  `v.into_iter().step_by(2).step_by(2).rfold(..)` drops a `Vec`'s items
  as one slice, front to back, as std's `nth_back` does. A chain carries
  no `ownNthBack`: it is never `ExactSizeIterator`, so no `step_by` steps
  it from its back.
- **`$enumerate`'s `nth_back` asked its source's length at its end.**
  Std's is `let a = self.iter.nth_back(n)?; let len = self.iter.len();`,
  so a source's `None` returns before the length is asked. The runtime
  now asks it only of an item found.
- **The `OwnStep` catalog had no back steps for `StepBy`.** `asked_of`
  now says that `StepBy`'s `nth_back` asks its source's (`NthBack`, like
  `Skip` and `Take`), and that its `rfold` and `try_rfold` ask its
  source's `nth_back`. Without these rows, `--std-iterators` gave generic
  code a `step_by(2)` of the crate's iterator of `u32`s with no compiled
  `rfold`, so it was folded by `next_back` and asked the crate's
  `size_hint` at each step, unlike native Rust. Items with a destructor
  hid it, as std's drop order makes the compiled `rfold` given anyway.

The proof:

- All 15 batches that hold #834's rows, and every batch of the matrix and
  conformance table with a `step_by` row (505 batches, 17,367 rows), run
  as native Rust does in the default build, with refused cases set aside
  as the harness does. The generic `StepBy<I>` rows are refused, as they
  were before #814, and the `--std-iterators` build runs them all.
- Corpus `step_by_folded_from_its_back_as_std` holds each shape against
  native Rust in both builds. It is also in `iterator-forms`' run of a
  few, which holds the catalog's rows in the `--std-iterators` build.
- Mutations: `step-by-ends-rfold-by-back`,
  `step-by-ends-own-nth-back-unmarked`, `enumerate-own-nth-back-unmarked`,
  `enumerate-nth-back-len-asked-at-end`, `skipping-own-nth-back-unmarked`,
  `taking-own-nth-back-unmarked`, `std-iterator-nth-back-unmarked`,
  `step-by-rfold-asks-no-nth-back` and `step-by-nth-back-unowned`. Five
  older mutations whose finds the new code repeats now match once.

## Amendment: what the points table found first (2026-11-06, #843, #844, #845)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Each changed results,
so Rust's behavior is kept.

The points table (ADR 1090's #817 amendment) panics at each point of each
case in turn. It found three runtime frames that dropped what std's drop,
or dropped what rustc leaks, as one of those points unwound:

- **A collect in place drops what it collected as its source's rest's drop
  panics** (#843). `$foldToArray`'s in-place path closed what it folded
  inside the `try` whose `catch` is for the fold's own panic, whose
  `unwind` has already dropped what it collected, so a throw from the close
  dropped nothing. A `take()` also closed its source at its limit, inside
  the fold. It now holds what it folds (`hold()`, S9), then closes the
  source whose buffer it reuses, the root of the adapters' `from[0]`
  (`$inPlaceSource`), under the guard std's `InPlaceDstDataSrcBufDrop`
  is: a panic there drops what it collected, after the rest. Then it closes
  the adapters with no guard: by then std's `from_iter` has made the `Vec`
  its result, which rustc leaks as an adapter's function's drop panics
  (rust-lang/rust#47949). `$collectEnded` is the one end of a collect,
  `$collectDropping`'s too. A compiled struct has no `from`, so its whole
  close is guarded: right for its source's rest and wrong for its function,
  which is #852, listed in iterator-forms' `differs`.
- **A `skip()` drops the item its first `nth(n - 1)` gives back inside
  that step** (#844), as std's `Skip` drops it in the frame that holds
  `self`, `init` and `f`: its `fold`, `try_fold`, `count` and `last`, and
  `$lastDropping`'s skip, which now closes it as that step throws. The
  item's drop ran after `$stepHolding` and `$consumed` had returned, so its
  panic leaked the accumulator and the rest.
- **A `step_by()` folded by value holds what it steps** (#845), so its
  source is closed as the fold ends, not inside the step that finds it
  done. There, the source's drop's panic made that step's `unwind` drop the
  accumulator, which std's fold has made its result by then and rustc
  leaks. `$stepBy`'s and `$stepByEnds`' `fold` and `rfold`.

Corpus cases, both builds, against native Rust:
`collected_in_place_dropped_as_the_rest_panics.rs`,
`collected_in_place_leaked_past_an_adapters_drop.rs` (the default build's
alone, #852), `skip_passed_item_drop_unwinds.rs` and
`step_by_source_dropped_at_its_folds_end.rs`. The points table's 166
case-builds under #843–#846 are off its list; #852's 4 are on it.
Mutations: `in-place-collect-unheld`, `in-place-close-unguarded`,
`in-place-guard-over-the-adapters` (`runtime/fold_of.ts`),
`skip-passed-item-dropped-outside-its-step` (`runtime/skipping.ts`),
`last-dropping-skip-unclosed` (`runtime/last_dropping.ts`),
`step-by-fold-unheld` and `step-by-rfold-unheld` (`runtime/back_steps.ts`).
`fold-to-array-collected-last`, `collect-end-unwound-leaks`,
`in-place-by-fold`, `consume-unheld` and the skip mutations of #492 and
#768 name their lines as they now read.

## Amendment: what a helper drops as a panic unwinds is one combinator's (2026-11-06, #817)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The JS a program
compiles to is the same, but for a `skip_while`'s, the user's choice of form,
and its behavior is the same, but where a second panic unwinds (below).

The panic and drop column of this contract had no owner. Each of 40 runtime
helpers wrote its own `try`, `catch` or `finally` to drop what it held as a
panic unwound: `$consume`, `$consumed`, `$staged`, `$kept`, `$stepHolding`,
`$endsAfter`, `$closing`'s fold, `$holding`'s, the collects, `$lastDropping`,
`$extremeBy`, the zips, three copies of `FlattenCompat::fold`'s `finally {
ends() }`, `$partition`, `$extend`, `$reduce`, the comparisons, the capacity
hints and more. Each kept its own order and its own answer of which error
goes on. The points table found the frames that drifted (#843, #844, #845).

- **`$unwinds(run, ...drops)`** (`steps.js`) is the one place a helper drops
  what it holds as a panic unwinds: `run()`, a call std's frame makes while
  it owns what `drops` drop, given in that frame's order, its locals'
  reverse order. Where it throws, each drop runs in turn, the next though one
  throws, and the panic goes on. The frames of std's are named once on top of
  it, each citing its std source: `$staged` (a fold closure holding the
  accumulator), `$kept` (a discarding closure: the item, then the
  accumulator), `$stepHolding` (a by-value fold before its accumulator is a
  local: `f`, then `init`), `$consumed` (`f`, then `self`), `$endsAfter`, and a
  collect's `$collectUnwound` and `$collectEnded`.
- **`$inTurn(...runs)` and `$inTurnOf(items, run)`** are the one "each though
  one throws" of a frame's end and of drop glue, the rest run as the first
  throw unwinds. `$dropItems` is `$inTurnOf`'s.
- **Std's default fold is one loop, `$foldBy(it, "next" | "back", ..)`**, the
  fallback a consumer reaches after the value's own `fold` or `rfold`
  (m6-lily's review: never a loop ahead of a step that exists, #834's
  shape). `$Steps`' `fold`, `$foldDropping`'s and `$foldBack`'s were three.
  `$foldDropping`'s dropped what drops with `f` after what's left, where std
  drops `f` first. That was latent: only a bare `$iter`, which closes
  nothing, reached it.
- **A drop that throws while a panic unwinds is `$dropsUnwinding`'s alone**,
  which #847 makes `$cleanup`'s: Rust's abort (the user's decision,
  2026-11-06; ADR 0660, amended), where #817's `$unwindingThrew` kept ADR
  0660's earlier answer, the first error going on.
  `$consumed`, `$stepHolding`, `$foldBack` and `$collectUnwound` had let the
  later drop's error replace the panic, and `$dropItems` the last error of
  many. Native Rust aborts in every such case, so no native comparison can
  tell them apart.
- **A `skip_while`'s test is the runtime's**, `$lazySkipWhile(it, skip,
  drop)`, as a `take_while`'s is: the lowering's `dropping_skipped` wrote a
  `try`/`finally` copy of `$kept`'s rule into each test it emitted. Eleven
  corpus cases are 102 lines shorter.
- **A zip's `SpecFold` fold** is `$foldBy` of a view that ends at its shorter
  side's length. It no longer patches its own `next` and restores it in a
  `finally`.

`test/architecture-iterators.test.ts` ("a runtime helper drops what it holds
as a panic unwinds by $unwinds alone") refuses a `try`, `catch` or `finally`
in `src/runtime/` outside `$unwinds`, `$dropsUnwinding` and `$inTurnOf`,
but of 21 helpers it names, each with why: those that aren't an iterator's,
whose unwinding #854's panic points will check. It also refuses a
`StmtKind::Try` built by `lower/iterators/` or `lower/mir/iter/`. The
combinators' own emitted unwinding (`combinators.rs`'s `comb_values`,
`combinators/values.rs`, `adapters.rs`'s `inspect`) is the lowering's and
is left for later. 60 mutations name their lines as they now read.
`unwinds-drops-nothing`, `skip-while-throw-keeps-item` and
`skip-while-skipped-undropped` are new. Three are gone as equivalent:
`holding-fold-unheld` (it already survived on main),
`last-dropping-kept-dropped-twice` and
`last-dropping-flatten-kept-dropped-twice`, because a frame's `unwind` runs
only as a step throws, never after `f` has.

## Amendment: an iterator's own facts and its lifecycle each have one writer (2026-11-07, #870)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The JS a program
compiles to is byte for byte the same. The runtime's answers are the same.

**The own facts.** §4's `ownFold`, `ownRfold`, `ownNth`, `ownNthBack`,
`ownLast` and `ownCount` were written in 12 places. Seven adapters copied
their source's facts by hand, each choosing a subset (`if (iter.ownNth)
skipping.ownNth = true`). Compiled code reads the facts by these names
(`mir/iter.rs`'s `!iter.ownFold` of a type parameter given no dictionary).
The names are therefore kept, and every write now goes through
`forward.js`, the facts' owner:

- `$owns(it, ...steps)`: a maker's own facts, said where it makes the
  iterator. These are `$iterator`'s (the crate's), `$slicing`'s (a `Vec`'s
  slice steps), `$stdIterator`'s (compiled `nth` and `last`, and `nth_back`
  once a use site gives it), a flatten's `count`, a `$closing`'s slice steps,
  and a chain's, which combines its sides'.
- `$ownsAs(adapter, source, ...steps)`: an adapter's own step that passes
  it on to its source's step of the same name. That step is the adapter's
  own exactly where the source's is. `$forward`'s step passed on by name
  says so through `$owns` too.

`test/architecture-iterators.test.ts` ("an iterator's own facts are written
by $owns alone") refuses a write of `.ownX` or `[$ownStep(..)]` in any other
runtime file. Gap G8's reads stay reads by name. 22 mutations name their
lines as they now read. `owns-as-unsaid` is new. One mutation survived the
corpus and the conformance slice and is not kept: `$ownsAs` saying a step is
its own where its source's isn't. Each adapter's step that passes on also
passes on the items' drop, so where the source's step drops by it the
false fact changes nothing.

**The lifecycle.** A `$closing` and std's compiled struct each drop what
they hold once. Three marks recorded where each was: `$closing`'s local
`closed`, set at 6 sites, std's struct's `it.$closed`, and a fold's
`closed = true`. The marks disagreed in when they were set: an own `fold`
and `rfold` were marked after their step and an own `count` and `last`
before it. `closed` also stood for two things, dropped and taken. One field,
`$state`, now holds the lifecycle. It changes only through `closing.js`'s
transitions:

| From | `$close(it, drop)` | `$takenBy(it, run)` |
|---|---|---|
| open (none) | closed, `drop()` once | taking while `run()` runs, then taken, though it throws |
| taking | **internal error**: a drop of what its own consumer holds | **internal error** |
| taken | nothing: its consumer dropped it | **internal error**: a second consumer |
| closed | nothing | **internal error**: a consumer of what's dropped |

`$stateFrom(target, value)` carries the state with a move of std's struct
or a write of the whole struct (#537). A `$closing`'s own `fold`, `rfold`,
`count` and `last` are `$takenBy`'s, so all four are now marked after their
step. The old order of `count` and `last` was never observable. Their
crate step is given what the `$closing` holds, never the `$closing` itself,
and a panic marks it taken before anything else runs. No corpus case,
points row or conformance row closes one while it runs: each would now be
an internal error, and none is.

Std's compiled body is the struct's own code, and it drops `self` by its
own `return()`, as `Take`'s `spec_fold` does. Its `$takenBy(it, run, true)`
(`closesItself`) stays open while `run()` runs, so that `return()` drops it
once, and is then taken. The first build without `closesItself` failed 16
corpus cases, every one this way, and no other move fired.

An impossible move is rust-js's bug, the user's decision (2026-11-07). In
every build it throws `$InternalError` ("rust-js internal error: $closing
moved from taking to closed"), defined in `unwind.js`. That is not a panic
(`$isPanic`): `$catchUnwind` rethrows it as it rethrows a `$Abort`, and
`$cleanup` rethrows it rather than aborting, so a compiler's bug is never
caught as a program's. Checking costs one comparison per close or take,
never per step. `test/runtime-package.test.ts` ("a lifecycle's impossible
move is rust-js's internal error, never a panic") makes each move, possible
and impossible, since no program can reach the impossible ones.
`own_count_and_last_that_panic_drop_once` is new: the crate's own `count`
and `last`, panicking through a `chain`, a `skip` and an `enumerate`, are
dropped once. Unmarked, the chain's close as it unwinds dropped them again,
which no earlier case pinned. `closing-own-count-closed-again`,
`closing-own-last-closed-again`, `close-while-taking-unchecked`,
`taken-again-unchecked`, `state-from-unset`,
`std-iterator-body-closes-itself-unchecked`,
`catch-unwind-catches-internal-error` and
`cleanup-aborts-on-internal-error` are new. Six name their lines as they
now read.
