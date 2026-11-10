# 0419. A `Vec`'s capacity is kept where it's asked

Status: Accepted. Amends [0315](0315-vec-capacity-and-edits.md), which
refused `capacity()`. Amended by
[0774](0774-room-past-isize-max-bytes-panics.md): the room asked is checked
first; and below (#849, #1180, #1200, #1215), its number std's growth's.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): `capacity()` is
printed, so results change; its number is std's growth's (#1180, the
user's decision; it was case B, its own number, which goes).

## Context

JS's array has no capacity, and ADR 0315 refused `capacity()`: a number
made up would break what std promises of it, `Vec::with_capacity(10)`'s
`capacity() >= 10`, and that `clear()` leaves it. smallvec asks it, in
`from_vec`, to keep a `Vec`'s room as its own: refused, smallvec was.

## Decision

**A `Vec`'s capacity is kept, `$capacities`, where the crate asks it, or a
library it uses does: at least its length, what `with_capacity` and
`reserve` gave, and never less where it's shortened.**

| Rust | JS, where kept | JS, where not |
|---|---|---|
| `Vec::with_capacity(n)` | `$withCapacity(n)` | `[]`, as before |
| `v.capacity()` | `$capacity(v)`: its kept room, or its length, the more | (it's kept, as it's asked) |
| `v.reserve(n)`, `reserve_exact(n)` | `$reserve(v, n)` | nothing |
| `v.shrink_to(n)`, `shrink_to_fit()` | `$shrinkTo(v, n)`, of `0` | nothing |
| `clear()`, `truncate`, `set_len`, `pop`, `drain`, … | `$keepCapacity(v)` first | as before |

- **Which `Vec` methods ask or change its capacity is
  `recognition::capacity_op`'s**: `with_capacity`, `reserve` and the rest
  are its alone, where it isn't kept too, which ADR 0315's arms of them
  answered a second time, unreached, and are gone.
- **Which `Vec` methods shorten it is `recognition::shortens_vec`'s**,
  asked once, before any std call is lowered: what it has kept first.
- **Where it's kept is the crate's, `asks_capacity`, or a library's**: a
  library's manifest says it keeps capacity, `keeps_capacity`, and its
  consumers keep theirs, as a vector they give it may be asked. A crate
  that neither asks nor uses one that does is the JS it was.

## Why

- **It keeps std's promises**: `test/corpus/vec_capacity.rs` asks a
  `with_capacity`'s, a `reserve`'s, and one left by `clear()`, `truncate`
  and `pop`, as native Rust answers; `test/crates.test.ts` a library's of a
  consumer's vectors.
- **It costs only who asks**: a crate that never asks a capacity, and
  uses no library that does, gains nothing.

## Consequences

- ~~Its number isn't native Rust's~~: superseded by the 2026-11-10
  amendment below, its number std's.
- `String`'s and a map's capacity stay refused.

## Amendment: a `VecDeque`'s and a `BinaryHeap`'s

A `VecDeque` and a `BinaryHeap` are JS arrays too (ADR 0068), and their
capacity is their array's. Which collections are so is
`recognition::ARRAY_COLLECTIONS`, the one list: `with_capacity`,
`capacity()`, `reserve`, `reserve_exact`, `shrink_to` and `shrink_to_fit`
of each are `capacity_op`'s, and what shortens each, `pop_front`,
`pop_back`, a heap's `pop`, `clear`, `drain`, `retain` and the rest, and
`append`'s other, `shortens_vec`'s, keeping what it had. Their
`with_capacity` was their `new`, a second answer to what it is, and is
`capacity_op`'s alone. The `collection_capacity` corpus case asks a heap
made with room, reserved, popped and cleared, one grown by pushes and
cleared, and a deque so, with native Rust; rustc's
`zero-sized-binary-heap-push` passes.

## Amended (2026-11-06, #849): its number where an `extend` asks

Its number was its own, case B: but where an `extend` of the crate's own
`size_hint` asks it as a `Vec` fills, when it's asked is the capacity's,
and results change. So a `reserve(n)` grows it as std's `grow_amortized`
does, `reserve_exact(n)` as many as it needs, and `extend` as std's does
(`$grownBy`); a crate with such an `extend` keeps capacities as one that
asks them does. A `Vec` grown by `push` still has its length as its
capacity, so such an `extend` is exact only of one made in the same
function and only extended or reserved since, and refused elsewhere
(ADR 1097, the user's (B)).

## Amended (2026-11-10, #1180): its number std's growth's

**User decision: match std's growth.** `capacity()` is printed, so a
number other than std's changes results, which case B wasn't listed as
in ADR 0262 or the guide: case C. Where capacities are kept, each
`Vec`, `VecDeque` and `BinaryHeap` keeps the one std's growth gives it;
a crate that keeps none prints nothing more.

| Rust | std's growth | JS, where kept |
|---|---|---|
| `push`, `insert`, `push_front`, `push_back` | `grow_one`: `grow_amortized(len, 1)` | `$grew(v, size, v.length, v.push(x))` |
| `append`, `extend_from_slice`, `extend_from_within`, `resize`, `resize_with`, `extend` of a `TrustedLen` iterator | `reserve(added)` | `$grew(..)` of the call |
| `extend` of another | `extend_desugared`: where full, `reserve(lower + 1)` | `$grownBy(v, it, size)` |
| `collect()` of another | `SpecFromIterNested`: `max(MIN_NON_ZERO_CAP, lower + 1)`, then as `extend` | `$grownVec($roomForHint(..), collect)` |
| `collect()` into a `Result` or an `Option` of a `Vec` | the `GenericShunt`'s lower bound, none | `$shuntGrowth`'s, kept |
| `BinaryHeap::append` of a longer heap | the two swapped first | their capacities swapped |
| `capacity()` of items of no size | `usize::MAX` | `$capacity(v, 0)` |

`grow_amortized` is at least twice the capacity, as many as needed, and
`MIN_NON_ZERO_CAP`: 8 of 1-byte items, 4 up to 1 KiB, 1 above.

- **Which calls add to a `Vec` is `recognition::grows_vec`'s**, beside
  `shortens_vec`, and `extend_grows`'s of `Extend::extend`; the items whose
  size each grows by, `grown_vec_item`, the one answer of the lowering
  (`mir_std_call`, `capacity_item_size`) and the analysis
  (`grown_asked`), which asks a caller's `TSize` where they're a type
  parameter's.
- **Whether an `extend` grows item by item is `extend_fills`'s**: by an
  iterator whose hint runs the crate's own, or, where capacities are
  kept, that isn't `TrustedLen`. A dictionary's `extend`, of an iterator
  generic code gives, `$grownBy` alike where capacities are kept, which
  of a `TrustedLen` one, whose hint is exact, reserves what's left where
  it's first full, as std's reserve of them all leaves it.
- **Case B goes: `mir/known_capacity.rs` and its refusals are gone.** An
  `extend` asking the crate's own hint knew the capacity only of a `Vec`
  made in the same function and only extended or reserved since (ADR
  1097, the user's (B)); every `Vec` of a crate that keeps capacities is
  std's now, so each such `extend` compiles. Generic code's in a crate
  that keeps none stays refused at the call that gives it its types.
- **Refused where JS's hint hasn't std's lower bound**, by which std
  grows a `Vec` that isn't `TrustedLen`: an array JS counts of an iterator
  std doesn't, no `ExactSizeIterator`, a string's `chars()` or
  `split(..)` (`lengths.rs`'s `lower_bound_is_stds`), as `size_hint()` of
  them is, or a chain's part that's an exact-sized adapter (below, #1200); and a bound's `extend` of a `Vec<T>` of a type parameter's
  items, whose dictionary isn't given their size.

`test/corpus/vec_capacity_grown.rs` asks each, of items of 1, 2, 8 and
2000 bytes and none, against native Rust on wasm32;
`extend_asks_hint_capacity_grown.rs` the `extend`s case B refused.

The readability baseline is blessed: a renamed `x$1`'s share moved from
511 to 511.1 in 10,000 lines, of `length$1` and its kind, the length an
`extend` written as an index loop held before it (`$grew(v, size,
length, ..)`, ADR 1042), in a crate that reads a capacity, and the new
case's own names; no construct of them new.

Not yet, filed with #1180: a `collect()` std makes in place of a
`vec::IntoIter`'s buffer, whose capacity is its source's; a `Vec` a
`String` gives, `into_bytes()`, whose capacity is the string's, which a
JS string has none of; and a `Vec` a library that keeps no capacities
grew, given to a crate that reads one; and a `Vec` of items whose size no
caller gives, an associated type's, `I::Item`, or one built of a type
parameter, `vec::IntoIter<I::Item>` (`capacity_item_size`'s `None`), which
itertools pushes to: its growth isn't kept, as before, never refused, as
#934's facts of associated types will give it.

## Amended (2026-11-10, #1180): a capacity not kept is an error, never a number

**User decision: a `Vec` whose growth wasn't kept has no recorded
capacity, and `capacity()` of it throws a clear error naming #1180,
instead of silently returning its length.** Nothing that compiled stops
compiling. Of a crate that keeps capacities, `$untracked(v)` marks one:

| Made by | Why its capacity isn't known |
|---|---|
| a `collect()` std may make in place (`collects_in_place` not `Some(false)`) | its source's, `src_cap × src_size / dst_size`, which needs a MIR link from the collect to its source |
| `into_bytes()`, `into_boxed_bytes()`, `Vec::from(s)` of text | the string's, which a JS string has none of |
| a call that adds items whose size no caller gives, past what it had (`$grewUnsized`) | an associated type's, `I::Item`, till #934 gives its facts |

`capacity()` of it is `$InternalError`, "this `Vec`'s capacity isn't known
to rust-js: std's growth of it wasn't kept (#1180)", which no
`$catchUnwind` takes for a panic (the inline form's limit, ADR 0660, is
#1193's). What keeps a capacity carries the mark: `push` and the rest,
`reserve`, `reserve_exact`, `clear()` and what shortens it leave it
unknown; `shrink_to_fit()`, and `shrink_to(n)` of `n` at most its length,
know it again, its length. An `extend` that asks its hint as it fills,
`$grownBy`, steps one unknown without asking it, its hint's timing
unseen, and throws the same error where the hint is the crate's own
(`own`), whose asking is seen. `test/lowering-collections.test.ts` holds
each. Still silent: adapters of JS's own given to generic code that
collects them (below, #1180's (3)), and a `Vec` a library
that keeps none grew, whose growth no code of its marks; #1180 lists them.
The conformance table's 148 cases that read the capacity of a collect std
may make in place throw it now, listed under #1180 till its source's is
kept.


## Amended (2026-11-10, #1200): an `ExactSizeIterator` collected knows its length

Case C ([0262](0262-when-rust-and-js-disagree.md)): the capacity is printed,
so std's number is kept.

A `Vec` collected of a B-tree's or a hash table's iterator skipped,
`b.iter().skip(1)`, of 40 items, had capacity 64 where std's is 39.
`Skip<btree_set::Iter>` isn't `TrustedLen`, so std grows by its hint's
lower bound, which is its length as an `ExactSizeIterator`'s. Its JS was
JS's own `drop(n)` of an array's iterator, `$sortedKeys(b).values().drop(1)`,
and neither knows a hint, so `$roomForHint` doubled from 4.
`lower_bound_is_stds` trusted std's adapters over any source that isn't
an array's iterator; the same held for `enumerate()`, `map(f)`, `take(n)`,
`step_by(n)` and `rev()` after a `skip(n)`, and for `into_iter()` and a
map's `keys()`.

- **A collect of an `ExactSizeIterator` whose lower bound isn't always 0
  is given knowing its length** (`lengths.rs`'s `grows_by_exact_len`),
  through the owner of iterators that know their length, `as_exact_size`:
  `$skipping($iter(items), 1)`, `$enumerate`, `$mapEnds`, `$taking`,
  `$stepByEnds`, whose hints are std's. It isn't given that way where std's
  compiled struct is collected, whose own hint is std's, or where the hint
  runs the crate's own, which JS asks as it is. An array's `$iter`,
  a hash table's `$iter(Array.from(set))`, knows its length as it is.
- **What isn't one is refused where a part of it is an exact-sized
  adapter**, a `chain`'s or a `zip`'s, `a.iter().skip(1).chain(rest)`. The
  part is JS's own adapters of an array's iterator, and the whole isn't
  given knowing its length, so its lower bound is lost. This includes a
  `Vec`'s, which compiled with another capacity before. A chain of what
  it steps as an array's `$iter`, `b.iter().chain(rest).skip(1)`, keeps
  std's hint, as `$Steps`' `drop(n)` is `$skipping`'s.

`test/corpus/capacity_exact_size_adapters.rs` holds each at 0 to 40 items
against native Rust; `test/diagnostics.test.ts` holds the refusal. The
size hint generic code prints, `(0, None)` of these, is the guide's
difference 14 (ADR 0170), which this leaves as it is.

## Amended (2026-11-10, #1215): what std hints less of than JS counts

Case C ([0262](0262-when-rust-and-js-disagree.md)): the capacity is printed,
so std's number is kept.

`b.range(2..).collect()` of 38 items, in a crate that reads a capacity,
had capacity 38 where std's is 64. A B-tree's `Range` has no `size_hint` of
its own, so std's is `(0, None)`, and std grows by its lower bound, 0. Its
JS is the array of what's in range, stepped as `$iter(items)`, whose hint
counts what's left. The same held for a set's `intersection(..)` and
`symmetric_difference(..)` and a hash set's `difference(..)`, whose lower
bounds are 0 once an item is taken, which is when std asks.

- **Of what std hints no lower bound of, which JS counts, the `$iter` is
  `$iter(items, true)`**, whose hint is std's lower bound, 0, and what's
  left as its upper bound. `lengths.rs`'s `hints_no_lower_bound` lists
  them. The one maker,
  `stepped_items_of`, writes it where the crate keeps capacities: a
  source's `$iter`, a reversal's, made of its source's type, and a clone's. `lower_bound_kept` gives
  them `Some(false)`. `fresh_iter_source` reads either form.
- **What the merge's state gives a lower bound of is refused**: a B-tree
  set's `difference(..)` and `union(..)` and a hash set's `union(..)`,
  whose lower bounds change as they're stepped (`hints_merged_lower_bound`).
- **An exact-sized iterator over an array JS steps lazily, a heap's
  `iter()`, is the array's `$iter`** where the crate keeps capacities
  (`counted_where_kept`), so std's compiled `Skip` and `Enumerate` ask its
  length. `as_exact_size` takes an enumeration of what isn't an array's
  through its source given knowing its length. #1200 had refused
  a map's `values_mut()`, a hash set's or a heap's iterator enumerated and
  a heap's iterator collected; they compile.

`test/corpus/capacity_of_what_std_hints_less.rs` holds each at 0 to 40
items, built both ways (with std's iterator structs too), against native
Rust. `capacity_exact_size_adapters.rs` asks `b.range(2..).skip(1)` again.
`test/diagnostics.test.ts` holds the refusal.

## Amended (2026-11-10, #1180's (3)): a string's patterns hint as std's do

A string's `split(..)` and its kin, `rsplit`, `splitn`, `split_terminator`,
`split_inclusive`, `matches`, `match_indices`, `split_whitespace`,
`split_ascii_whitespace` and `lines`, have no `size_hint` of their own, so
std's has no lower bound. Their JS is the array of their parts, which
`$iter` counted, so generic code that grew a `Vec` of one it was given
reached another capacity, silently: `s.split(',')` of 6 parts, chained
with one more, 5 where std's is 8. They're `hints_no_lower_bound`'s, as
#1215's are: where capacities are kept their `$iter` is `$iter(parts,
true)`, whose hint is std's, and a `Vec` collected of one where it's made,
refused before, compiles with std's capacity. A slice's `split(..)`, whose
std hint's lower bound is 1, stays refused. `chars()` given to generic code
was std's already: its `$chars` hints as std's `Chars` does (ADR 1117).

`test/corpus/capacity_of_text_patterns.rs` holds them, given to generic
code and collected where they're made, against native Rust.

What's left silent of (3): an adapter of JS's own, `.drop(n)` or
`.map(f)`, made of what's then given to generic code, `chars().skip(2)` or
`b.iter().skip(1).copied()`, and one generic code makes of what it's
given, whose hints are none; #1180 lists them.

## Amended (2026-11-10, #1180's (3)): generic growth by a hint JS lost is unknown

What the previous amendment left silent of (3) is now loud, under the user's
decision above: a `capacity()` that would be wrong throws instead. Generic
code's `extend` of a `Vec` grows it by the hint of whatever its caller gives.
Where a caller gives an adapter JS makes of its own, the JS hint is none:
`chars().skip(5)`'s `drop(5)`, a `Vec`'s `into_iter().skip(1)`, or generic
code's own `it.skip(2)` of a string's `chars()`. The `Vec` then reached
another capacity silently, 64 where std's is 60. Where the type is known,
such an extend is refused at compile time (`refuse_unkept_lower_bound`).
Generic code's type, `Skip<I>`, passed that check, because `I` stands for
any type.

`hint_asked.rs` now asks each `extend` that grows a `Vec` by an iterator of
generic code's type parameters about each instance its callers give
(`Hinted::lost`). The site is lost where one instance's hint, as it's
given, has no lower bound std's has (`given_hint_is_stds`, which a
`TrustedLen`'s does) and doesn't reach the crate's own (`hint_reaches_own`),
which runs as it is, a `map` of its own exact-sized iterator's too. A bare
`chars()` given is `$chars`, whose hint is std's, and so is its `rev()`.
`$grownBy(.., lost)` grows such a `Vec` as its JS hint
says, so a hint that's the crate's own is still asked at std's moments.
Once it ends or is closed having asked its hint, the `Vec` is `$untracked`,
and `capacity()` throws the #1180 error. A `Vec` that never grew there,
with room enough or no items, keeps its capacity.

A lost site's JS function is every caller's, so a caller whose hint is
std's loses its capacity there too. That's loud, not wrong. Generic code's
collects were already untracked as collects std may make in place. ADR
[0262](0262-when-rust-and-js-disagree.md)'s third case applies: results
changed, so where JS can't keep std's capacity, `capacity()` refuses.
`test/lowering-collections.test.ts` holds the lost sites and the kept ones
against native Rust's numbers.

Where callers can't be seen, as a library's `pub` function's can't, the
site isn't lost by its type. Each library's generic `extend` would be, and
a `Vec` then extended again by the crate's own hint would throw at the
`extend`, where native runs.

Instead such a site is asked of each value it's given (#1251,
`Hinted::by_value`), as it grows: `$grownBy(.., byValue)` asks its hint
through `$hintAsked`. That records whether the hint met JS's own iterator,
which knows none: a string's or an array's `drop(n)`, or a slice's `split`
given as JS's array iterator. Every form of rust-js's own is a `$Steps`
whose hint is std's, its default `(0, None)` where it keeps none. Where
the hint met JS's own, the `Vec` is lost as above. So a consumer's
`chars().skip(5)` given to a library's `pub fn grow_any<I>(..)` throws at
`capacity()`, while the crate's own iterator, a range, a filter or a bare
`chars()` keep std's. The evidence is the value's own, with no flow and
no second owner of what a hint is.

Still silent of #1180: (5), a `Vec` a library that keeps no capacities
grew.

## Amended (2026-11-10, #1180's (5)): a library that keeps none

A `Vec` grown by a library that keeps no capacities has std's capacity in a
consumer that reads one. The consumer compiles its own copy of each of
that library's functions it reaches (ADR
[0914](0914-a-librarys-generic-function-is-its-programs.md), amended), by
the user's decision. #1180's list of what's silent is empty.
