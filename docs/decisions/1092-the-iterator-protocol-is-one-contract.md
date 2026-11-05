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
| `tryFold(init, f, broke, unwind)` | `try_fold` | `&mut` | `next` until `None` or a break, `iterator.rs:2482` |
| `rfold(init, f, unwind, ends)` | `rfold` | by value | `next_back` until `None`, `double_ended.rs:353` |
| `tryRfold(init, f, broke, unwind)` | `try_rfold` | `&mut` | `next_back` until a break, `double_ended.rs:282` |
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
`skip()`) run their first `nth` outside `$staged` and `$consumed`. As that
panics they leak `init` and never close the source. `$skipping`, the other
`Skip`, does both.

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
else std's default. An adapter's hint is std's for that adapter
(`Take::size_hint`, `take.rs:73`; `Skip`'s, `skip.rs:91`; `StepBy`'s;
`Zip`'s, ADR 0989), and a `Peekable`'s default `len` is its struct's hint
(ADR 0918's #706 amendment).

**S7. `return()` is the drop.** It closes once: a second `return()` drops
nothing (`$closing`'s `closed`, `$stdIterator`'s `$closed`). It drops the
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
closes `a`, then `b`.

Rust aborts on a panic raised while unwinding. JS can't abort. A drop that
panics during another's unwind goes on in JS, and the first error
propagates. No case under this contract depends on that abort; the
conformance table panics once a case.

**S8. Done is not dropped, but a temporary is.** A source an adapter holds
is closed once it's done, from either end or by `nth` (ADR 0570), unless it
is `held`, when its owner drops it where Rust does (#206). A temporary's
end and its source's end are the same statement, so no program sees the
difference (N).

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
| `StepBy` | `next:59` (its first `next`, then `nth(step - 1)`), `nth:69`, `fold:82`, `try_fold:73`, back steps | the source's `nth` and `nth_back` |
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
| `Chain` | `$lazyChain`, `$chainEnds` |
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
| `ownFold`, `ownRfold`, `ownNth`, `ownNthBack`, `ownLast`, `ownCount` (`$ownStep(step)`) | calling this step runs one that drops what it passes, or takes and drops the value, itself: the crate's own, a `Vec`'s slice step, or a compiled body | `$iterator` (the crate's own), `$slicing` (a `Vec`'s), `$stdIterator` (compiled `nth`, `last`), and `$forward` for a step passed on by name, with no `each`, from a source that has the fact | `$skipping`, `$stepBy`, `$closing`, `$took`, `$lastDropping`, `$count` |
| `dropItem` | the items' drop that an `ownNth` with none of its own drops by | `$slicing`, `$closing`, `$skipping`, `$lazyChain`, `$rev`, `$Steps.take` | `take`'s `nth`, `$lazyChain`'s `nth` |
| `held` | its owner is a variable or a temporary (S8); passed on to `from` and `iter` | `$held`, `hold()` | every done-closing |
| `from`, `iter` | what it steps, for `hold` and for `$knowing`'s search | each adapter | `hold`, `$knowing` |
| `back`, `len`, `hint` | the traits it has (S6) | each adapter, of its source's | `$rev`, `$taking`, `$enumerate`, `$sizeHint` |
| `of` | the value `$iterator` steps: the crate's struct, a range's bounds | `$iterator`, and `$rev` passes it on (ADR 0545's #721 note) | the iterator's drop (`drop(it.of)`), clones, `std_types/range.rs`'s `count`, `last` and `nth` |
| `items`, `at`, `end`, and its own `advanceBy` and `advanceBackBy` | a `Vec`'s, an array's or a `VecDeque`'s `IntoIter`, which drops what it passes as one slice | `$iter`, `$slicing` | `$closing`, `$given`, `$advanceSlice` |
| `$steps`, `$steps.takes`, `$closed` | std's compiled struct, the steps that take it, whether it's taken | `$stdIterator` | `$took`, `$lent`, `$closing` |
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
  body that takes the struct (`steps.takes`) marks it `$closed` once the
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
1090's generator, built from the same sources, adapters and consumers, and
run by the same harness (`test/iterator-conformance.test.ts`). Where the
matrix pairs two adapters over a few consumers, this table takes one
adapter at a time and runs it through every step of §2 that it has:
`next`, `next_back`, `fold`, `rfold`, `try_fold`, `try_rfold` (`rfind`),
`nth`, `nth_back`, `count`, `last`, `size_hint` and `len`, and the drop of
what's left after a partial step. Each case runs:

- plain;
- with an item's drop panicking;
- with the adapter's closure panicking, and with the consumer's;
- with closures that capture a `D`, so S2's order shows (#734).

Each runs natively and in both builds. The `--std-iterators` build is the
second oracle, as (c) says. The adapters are every one §3 lists, including
those the matrix leaves out (`take_while`, `skip_while`, `map_while`,
`scan`, `inspect`, `filter_map`, `flat_map`, `by_ref`, `cycle`). They run
over the sources that tell its steps apart: a `Vec`'s, a range, the crate's
iterator that writes every step and one that writes none, and a type
parameter. Its known failures are listed as the matrix's are, under their
issues, and only shrink.

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
  against. Its gaps G1–G9 are #730's work list. G1 is #734, G2 is #735 and
  G6 is #736, each P0, filed from this contract's probes.
- docs/architecture.md's "Libraries and runtime" names the contract and its
  owners.
- The stale references to "ADR 1092" for the iterator matrix
  (`test/iterator-matrix.ts`, ADR 0510) now name 1090, its number.
