# 0940. `collect()` into a `Vec` asks the room its iterator's hint says, as std's `SpecFromIter` does

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0774](0774-room-past-isize-max-bytes-panics.md). Amended: what has no
end once it gives an item (below); amended: what isn't `TrustedLen` is
stepped once before its hint is asked (below, #724), and its hint is
asked again as the `Vec` grows (below, #728); a heap's and a B-tree's
collect is the `Vec`'s it's made of (below, #797).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
`(0..).map(f).collect::<Vec<u8>>()` called `f` without end where Rust
panics `capacity overflow` at once, in the normal build too; and with
std's structs alone, so did `(0..usize::MAX).map(f).collect()`, which the
normal build's `$range(0, n, 4)` checks.

## Context

std's `Vec::from_iter` (`SpecFromIter`) asks room before it steps what it
collects: a `TrustedLen`'s upper bound, a `capacity overflow` where it has
none, another's lower bound and one more. rust-js asked it of a range's
`collect()` made in place alone (ADR 0774). A JS chain made in place knows
no hint, but its type may say it has no end: a `RangeFrom`, a `repeat()`,
through what keeps each item.

## Decision

- **`collect()` into a `Vec` of std's struct asks the room its hint says
  first**, `$roomForHint(it, size, trusted)`, `trusted` as rustc selects
  `TrustedLen` (`is_trusted_len`); but of one that gives no more than an
  array's items it steps, which exist already (`bounded_by_items`).
- **Of a `TrustedLen` whose type has no end (`endless_by_type`), however
  it's made, the same**: its hint's upper bound, none, a `capacity
  overflow` before any item's made.

## Consequences

- ADR 0908's `differs` is 5, from 6: `asked_length_overflow.rs` runs as
  native Rust does with std's structs alone, and now checks an endless
  `collect()`, made in place and held, in either build.
- The normal build's JS: 15 cases' held structs collected ask their hint,
  `$roomForHint(c, 4, false).toArray()`, 109 lines in, 47 out.
- Mutations: `collect-hint-room-unasked`, `endless-collect-unchecked`,
  `endless-room-by-lower` (taken off with its code, 2026-11-11, below).

## Amendment: what has no end once it gives an item (2026-10-30, #433)

Case: C, results change. `[1].iter().cycle().cloned().map(f).collect::<Vec<u8>>()`
called `f` without end. Rust calls it once and panics `capacity overflow`.
A `Cycle` isn't `TrustedLen`. std's `SpecFromIter` steps it once, then
asks room for its hint's lower bound and one more. A cycle of what has
items gives `usize::MAX`, so `with_capacity` panics for items of any
size.

- **`collect()` into a `Vec` of what isn't `TrustedLen` but has no end once
  it gives an item (`endless_once_stepped`, `recognition/lengths.rs`)
  steps it once, then panics**:
  `$roomForHint(it, size, false, true, drop)`. That item is dropped as
  the panic unwinds, `drop` of its type, then what's left is closed. One
  with nothing collects nothing.
- **What has no end once stepped:** whatever `endless_by_type` finds;
  a `cycle()` of what has a lower bound above 0 wherever it has items
  (`lower_bound_kept`), which is an `ExactSizeIterator`, a `TrustedLen`, or
  a chain of these; a `skip(n)` of what has no end, since its `n` is spent
  by the first item. Each counts through std's adapters that keep every
  item, through a chain of two such, and through a zip of two such.
- **Not one with a lower bound of 0 or 1.** A `filter` before or after a
  `cycle()`, or a `successors`, makes std ask for a little room and go
  on, as JS does. So does a zip with what has an end. Items of no size
  have room for any number.
- **Refused where only what it holds as it runs says:** a `cycle()`
  chained with what has an end, which has no end only where the cycle has
  items; a `cycle()` of what's partly filtered, whose lower bound counts
  only the unfiltered part; a `step_by(n)` of what has no end, whose lower
  bound `usize::MAX / n` panics or not by `n` and the items' size. Any of
  them ran without end before.

Consequences: `collect_endless_once_stepped.rs` runs as native Rust does.
Three cases hold the refusals: `collect_cycle_chained_refused.rs`,
`collect_cycle_partly_filtered_refused.rs` and
`collect_step_by_endless_refused.rs`. No other case's JS changed. The
mutations are `lengths.ts`'s seven, `endless-once-collect-unchecked`,
`endless-once-of-items-of-no-size` and `endless-once-item-drop-ungiven`
in `collect.ts`, and `endless-once-unstepped` and
`endless-once-item-undropped` in `capacity.ts`.

## Amendment: what isn't `TrustedLen` gives its first item before its hint is asked (2026-11-05, #724)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. What's stepped and
what's dropped as a hint panics changes, so Rust's behavior is kept.

Std's `SpecFromIterNested::from_iter` of what isn't `TrustedLen` takes
the first item by `next`. Of none, it gives an empty `Vec`, asking and
stepping nothing more. Of one, it asks `size_hint` for room for its lower
bound and one more. `$roomForHint` asked the hint before anything was
stepped, the order of a `TrustedLen`'s. A crate's iterator whose
`size_hint` reads what `next` changes, or panics, showed it:
`Own { at: 0, .. }.fuse().collect::<Vec<D>>()`, the hint panicking,
printed `hint at 0` where native prints `next at 0`, `hint at 1`, then
drops `D(1)` and the iterator.

`$roomForHint` of what isn't `TrustedLen` now takes the first item by
`next`, then asks its room. It gives a `$Steps` that gives that item
first, then steps what it was given, closing it as it's closed. Of no
first item, it gives that `$Steps`, done, which steps nothing again. Where
asking the room throws, the item is dropped, then what it was given is
closed, as `from_iter` drops its locals as it unwinds. The compiler gives
it the items' drop wherever it isn't `TrustedLen`, not only where it has
no end once stepped. Where whether it's `TrustedLen` is a type
parameter's, its caller's `ITrustedLen` (#670), `$roomForHint` is given
that answer too, so of a caller's `vec::IntoIter` it asks the hint first,
as `extend_trusted` does, and of another it steps first. The amendment above is this case's: of `endless`,
the room asked after the first item is `usize::MAX`'s.

Left as it was: std's `extend_desugared` asks the hint again each time
the `Vec` is full as it grows, which the amendment below models. And a
collect the recognizers make in place, of the crate's iterator, asks no
hint at all, #733's.

`collect_closes_what_its_hint_panics.rs` holds a `fuse()`'s and a
`peekable()`'s collect of the crate's iterator that prints its steps and
hints: one that ends, one with no item, and one whose hint panics,
against native Rust in both builds; it joins iterator-forms' few (TESTS.md:
64). Mutations: `room-for-hint-asks-before-next` and
`room-for-hint-steps-none-again` (`runtime/capacity.ts`);
`endless-once-item-undropped` and `endless-once-item-drop-ungiven` (its
`lower/mir/iter/collect.ts`), each of the first item's drop, now held by
both cases; `endless-room-by-lower` and `endless-once-unstepped` follow
the code.

## Amendment: the hint is asked again each time the `Vec` is full, as it grows (2026-11-05, #728)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. How often the
crate's `size_hint` runs, and what's dropped as one panics, changes, so
Rust's behavior is kept.

Std's `Vec::from_iter` of what isn't `TrustedLen` (rustc 1.99.0's
library/alloc/src/vec/spec_from_iter_nested.rs:18, `from_iter`) starts
the `Vec` with room for `max(MIN_NON_ZERO_CAP, lower + 1)`, then
extends it by `extend_desugared` (vec/mod.rs:4092). That steps each item
by `next`, and where `len == capacity`, asks `size_hint` again and
reserves its lower bound and one more. `reserve` grows by
`grow_amortized` (raw_vec/mod.rs:503): to `max(capacity * 2, len +
additional, MIN_NON_ZERO_CAP)`. `MIN_NON_ZERO_CAP` is
`min_non_zero_cap(size_of::<T>())` (raw_vec/mod.rs:158): 8 of items of
1 byte, 4 of up to 1024, else 1. Items of no size have a capacity of
`usize::MAX`, so the `Vec` is never full. A crate's iterator whose
`size_hint` prints shows each ask: of 20 bytes, at 1, 9 and 17; of 20
`u32`s, at 1, 5, 9 and 17; of 5 items of 2000 bytes, at 1, 2, 3 and 5.

ADR 0419 keeps no native capacity on a `Vec`. That stays true of the
`Vec` the program holds. What's modelled here is the capacity the collect
itself has as it fills, inside the `$Steps` that `$roomForHint` gives: it
counts what it has given, and where that reaches the capacity, asks the
hint again and grows as std does, `$roomFor` checking the new capacity's
size. Where that ask throws, a panic, the item just taken is dropped,
then `$collectDropping` drops what's left, then what it collected, as
`extend_desugared`'s frame, then `from_iter`'s, unwind.

Corpus case `collect_asks_hint_as_it_grows.rs`: `u8`s, `u32`s with no
lower bound, with half of what's left and with all of it, 2000-byte
arrays, `()`s, a `peekable()`'s `u16`s, and items with a destructor
whose hint panics as the `Vec` grows. It runs against native Rust in
both builds. Mutations (`runtime/capacity.ts`):
`room-for-hint-unasked-as-it-grows`, `room-for-hint-least-of-any-size`,
`room-for-hint-first-capacity-least`, `room-for-hint-zero-size-fills`,
`room-for-hint-grows-by-hint-alone`, `room-for-hint-grown-item-undropped`.

## Amendment: a collect in place asks no room (2026-11-05, #770)

Case R: the build of std's structs dropped what a `filter()`'s collect had
gathered where a dropped item's drop panicked, which native Rust leaks.

Std's in-place collect (`alloc/src/vec/in_place_collect.rs`,
`from_iter_in_place`) writes into its source's buffer by `try_fold`, the
items it wrote held by an `InPlaceDrop` that is the fold's accumulator. It
asks no `size_hint` of an iterator that isn't `TrustedRandomAccess`, a
`Filter`'s, and where a dropped item's drop panics inside the filter's
closure, the accumulator that closure gives back is leaked, as rustc leaks
what a function gives back while a later drop unwinds
(rust-lang/rust#47949; `$kept`, #651). 82ee82006 (#724) gave the collect's
iterator `$roomForHint` before `$collectsInPlace(..)` chose how it's
collected, so the in-place branch, `$foldToArray`, folded the room-asking
wrapper too, which drops what was collected as a step throws, as
`extend_desugared` does. Bisected: 9953b391f runs as native Rust does,
82ee82006 doesn't.

The in-place branch folds the iterator as it was before room was asked of
it; only the branch that collects by `next`, `$collectDropping`, is given
`$roomForHint`. No std source collected in place has a hint a program can
see, so asking none there changes nothing else (N).

`generic_collect_drops_rest_first.rs` is off iterator-forms' `differs` and
in its few. The mutation is `in-place-collect-asks-room`.

## Amendment: a heap's and a B-tree's collect is the `Vec` it's made of (2026-11-06, #797)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. The crate's
iterator collected into a `BinaryHeap`, a `BTreeSet` or a `BTreeMap` was
never asked its `size_hint`, where native asks at 1, 5 and 9 of 10 `u32`s,
so Rust's behavior is kept.

Std's `FromIterator` of each collects a `Vec` first and makes the
collection of it: `BinaryHeap::from(iter.into_iter().collect::<Vec<_>>())`
(collections/binary_heap/mod.rs), and a B-tree's sorts the `Vec` and
`bulk_build`s of it (collections/btree/map.rs, set.rs). So that `Vec`'s
collect, std's `SpecFromIter`, is the whole of how it's stepped: its hint
asked as it grows, collected in place where its source allows, and
unwinding as it does. A `VecDeque`'s and a `Box<[T]>`'s already were; a
`LinkedList`'s extends a new one and asks none.

**One answer, `Recognition::grows_as_vec`**: whether std's `from_iter` of
a type grows a `Vec`, of a `Vec`, a `VecDeque`, a `Box<[T]>`, a
`BinaryHeap`, a `BTreeSet` and a `BTreeMap`. The collect asks room only
of what it says, and a heap or a B-tree is collected as a `Vec` of its
items in its allocator (`collected_as_vec` of `vec_grown_for`), then made
of it: `$heapFrom(items, cmp)`, `new $BTreeSet(items)`. A B-tree, which
takes any iterable, is given the iterator itself where nothing else is
asked of it. serde_json's `Map` isn't std's and keeps its own collect.

Where no hint is asked, what changed is only the `Vec`'s own form:
`$heapFrom(Array.from($lazyRange(0, 5).map(f)), $cmp)` is
`$heapFrom($range(0, 5).map(f), $cmp)`, a B-tree's `$lazyRange(1, 4)` is
`$range(1, 4)`, and a heap of a `Vec`'s iterator of items with a
destructor that a panic may stop folds in place, `$foldToArray`, as std's
inner collect does (`panic_part_way_drops`).

A generic collect into a `BTreeSet<T>` of a type parameter's items asks
as a `Vec<T>`'s does, once, as their size isn't known: #796's.

Corpus case `collect_as_vec_asks_hint.rs`: the crate's iterator into each
collection, mapped, generic over `u32`s, and of items with a destructor
whose hint panics as it grows; and std's own iterators, whose hints no
program sees. It runs against native Rust in both builds. Mutations:
`heap-and-btree-not-grown-as-vec`, `heap-grown-as-itself`,
`btree-collected-as-array`, `linked-list-asks-room`,
`grows-as-vec-without-btree-set`.

## Amendment: a `TrustedLen`'s endlessness is its hint's, not its type's (2026-11-11)

Case ([0262](0262-when-rust-and-js-disagree.md)): no observable difference.
Two ways said a `TrustedLen` collect has no end: the type's
(`endless_by_type`, given as `$roomForHint(it, size, true, true)`), and,
since 22880c6c1 (#1239), its hint, a lower bound of `usize::MAX` and no
upper bound, which std's `TrustedLen` contract says of every such
iterator and which generic code, whose type its caller knows, reads. The
first was redundant: mutation `endless-room-by-lower`, which took it away,
survived on main.

**The hint is the one way** (it covers concrete and generic code alike).
The trusted branch's `if (endless) throw` is gone, and a trusted call is
given no `endless`: `$roomForHint($repeating(1).map(made), 1, true)`, not
`.., 1, true, true)` (the user's choice, 2026-11-11: an argument nothing
reads isn't emitted). `endless_by_type` still decides that a trusted
collect asks room at all, and `endless` is still given to one that isn't
`TrustedLen` and has no end once stepped. Asking the hint first is unseen:
no `TrustedLen` of std's runs the crate's code in its `size_hint`.

The evidence, each arm of `endless_by_type` collected with the throw and
without, in both builds, against native Rust: `RangeFrom` of `u8`, `u32`,
`u64`, `i8` and `char`; `repeat`; their `map`, `enumerate`, `inspect`,
`fuse`, `peekable` (and once peeked), `copied` and `cloned`; a `chain` of
either side, a `zip`, `by_ref()` of a local and of a field, `&mut`, an
`impl Iterator` returned, a map held and one stepped once; each panics
`capacity overflow` before any item is made, the same output either way.
The mutation is taken off with the code. That probe found #1358 (a
`String` of `char`s asks no room first) and #1359 (a `Box<dyn Iterator>`'s
collect asks none).
