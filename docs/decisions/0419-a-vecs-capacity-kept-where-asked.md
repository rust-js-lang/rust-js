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

## Amended (2026-11-11, #1216): a chain's or a zip's exact-sized parts know their hints

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted;
the capacity is std's.

#1200 refused a `Vec` grown by a chain or a zip with an exact-sized
adapter part, `b.iter().skip(1).chain(rest)`, in a crate that reads a
capacity. Its parts' JS was JS's own adapters, `values().drop(1)`, which
know no hint, so the chain's lower bound was lost. The runtime's chain
sums its parts' hints and its zip takes the fewer, as std's `Chain` and
`Zip` do. So each part is now made knowing its hint where it's made:
- `hint_asked.rs`' `mark_counted`, of each `collect` or `extend` of a
  `Vec` such a crate grows by a chain or a zip, makes each `skip(n)` and
  `take(n)` in it `$skipping` or `$taking`, in that function alone, as
  #1240's are: of every function, a `HashSet`'s `extend` by
  `repeat(x).take(3)` elsewhere lost the drop of what the `repeat` held.
- What each counts is made knowing its hint too, down to its source: an
  enumeration's `$enumerate`, the source's `$iter`. A map of the chain
  is `$mapEnds`.
- Not the chain or the zip itself, which needs no back.

`lower_bound_kept_by_parts` (`lengths.rs`) says which such iterators can
be made so: each part's lower bound std's as it is, exact-sized, or
another such chain or zip, or a copy or a map of one.
`hint_known_by_its_parts` checks it of what's grown, a variable's too,
down to each part's source, and refuses what isn't. `knows_its_length`
answers yes of a `skip(n)` or a `take(n)` made so. A chain part that's
neither, a string's `chars()` skipped, is refused as before.
`test/corpus/capacity_of_chains_of_exact_size_adapters.rs` has chains and
zips of a set's, a map's keys' and values' and a slice's skips and takes.
It also has a nested chain, an enumeration skipped, a map of a chain, an
`extend` and an empty set, all against native Rust in both builds.
Mutations `chain-of-exact-parts-refused`, `chain-parts-unmarked`,
`chain-part-sources-unmarked` and `counted-part-made-knowing-uncounted`.

## Amendment: `@rust-js/std`'s bodies keep what they grow, as its manifest says (#1339)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change, so
Rust's behavior holds. Whether `@rust-js/std`'s bodies keep a `Vec`'s
capacity was answered two ways. Its build kept none, `keeps_capacity`
false, so `BufRead::read_until`'s default's `extend_from_slice` was a bare
`push` loop. Yet a consumer's `unkept_by` exempted a builtin library and
trusted it, marking nothing `$untracked`. `read_until` into an empty `Vec`
then gave `4 4` where native Rust gives `4 8`.

**`@rust-js/std` is built keeping what its bodies grow, and its manifest
says so; a consumer trusts each library, std too, by that flag alone.** The
build's `keeps_capacity` is `std_bodies ||` the rest, as its `pretty_debug`
is. `unkept_by` asks only the manifest's flag, with no exemption for
`builtin`. The flag's other reading, "what the crate gives this library
may be asked, so keep yours" (`Foreign::keeps_capacity`), leaves std out:
it asks no consumer to keep its capacities, so no program gains `$grew`.

The std package changed seven lines of `io.js`: `$grew` around
`defaultReadUntil`'s two `push` loops, and `$keepCapacity` before
`Split::next`'s `pop`. Its manifest says `keeps_capacity: true`.

Corpus case `capacity_of_what_std_bodies_grow` (`read_until` into an empty
`Vec` and into one of capacity 2, and `split`'s items), against native Rust
in both builds. Mutations `std-bodies-keep-no-capacity`
(`lower/pipeline.ts`, by the committed package's test) and
`std-asks-consumers-keep-capacity` (`lower/library.ts`, form only); two
refitted.
## Amended (2026-11-11, #1338): a byte source's `read_to_end` reserves first

std's `read_to_end` of a `&[u8]`, a `Cursor<T>` and a `VecDeque<u8>` each
`buf.try_reserve(len)` all it adds before it adds it, `grow_amortized`'s
`max(cap * 2, len + n, MIN_NON_ZERO_CAP)`. The runtime's `$sliceReader`,
`$cursorReader` and `$dequeReader` pushed alone, so a `Vec` read into
from empty had its length as its capacity, `2` where native Rust has `8`,
known and generic alike. Each `readToEnd` now asks `$reserve(v, n, 1)`
first, the capacity's one owner. Case C of
[0262](0262-when-rust-and-js-disagree.md): `capacity()` is printed. Held by
`test/corpus/io_read_to_end_capacity.rs`, each source from empty, with
room, with too little and generic, against native Rust in both builds;
mutations `slice-read-to-end-reserves-nothing`,
`deque-read-to-end-reserves-nothing` and
`cursor-read-to-end-reserves-nothing` (`runtime/readers.ts`). What a body
`@rust-js/std` compiles grows, `read_until`'s, keeps std's growth too (#1339).

## Amendment: an `extend` by a `dyn` Rust made asks its hint (2026-11-11, #1377)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

#1359 made a collect of a `dyn Iterator` ask its hint; an `extend` by one
still didn't. std's `extend_desugared` asks the hint whenever the `Vec` is
full, so the crate's own `size_hint` behind a `Box<dyn Iterator>` ran
there, and of what has no end its lower bound, `usize::MAX`, panicked
`capacity overflow` once it was full, where the JS stepped it forever.
When it's full is the `Vec`'s capacity, which the crate keeps where an
`extend` may ask the crate's own (`extends_as_it_fills`). That pass reads
MIR, where a binding's mark, a value's fact, isn't: it follows the
`extend`'s argument back (`made_by_a_binding_in`), through moves and a
`map`, `filter`, `take` or `skip` of one, to the call that made it, and a
binding's is JS's own, whose hint is its contract's `(0, None)`. Each
other `dyn` is asked as a collect asks it (`hint_reaches_own_where`), and
`extend_fills` and `extended_grown` read the value's mark
(`iterators::binding_made`). So a crate that extends only by a binding's
iterator keeps no capacities, `push` as it was.

Held by `extend_by_a_dyn_iterator_asks_its_hint.rs` (a boxed own iterator
whose hint prints where std's asks it, a `map` of one, a boxed endless
range's `capacity overflow`, in a crate that reads no capacity) against
native Rust in both builds, js-collections' binding iteration, and the
mutations `extend-of-a-dyn-keeps-no-capacity` and
`binding-made-in-mir-never`.

## Amended (2026-11-11, #1263): generic code's collect into a `B: FromIterator` keeps std's capacity

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is preserved. `capacity()` of a `Vec` that generic code collected
into a `B: FromIterator` its caller gave threw #1180's refusal, wherever
the caller might collect it in place. Native Rust gives std's number.

`$collectVec` now keeps std's capacity for each way std's `SpecFromIter`
collects (`IVecCollect`, #1239), so the lowering no longer wraps it in
`$untracked`:

- **By `next`:** what `$roomForHint` reached as it grew, `$grownVec`.
- **By fold, a `TrustedLen`:** its exact length, as before.
- **In place:** its source's buffer's, which isn't kept, so it's
  `$untracked` there and `capacity()` refuses it, as it does for a collect
  made in place where its type is known.

Std grows such a `Vec` by its iterator's hint. So what a caller gives
generic code that asks the hint (`AsksHint`: a `size_hint()` or a
`collect()`), or gives a library's generic function, is made knowing it
(`mark_given`, one rule for both). This applies where it steps an array JS
counts (`is_array_iter`) and JS's own iterator would lose its hint as given
(`given_hint_is_stds`). A map of the crate's own iterator, whose
`$iterator`'s own `map` keeps std's hint, stays as it was. An example is a set's
`iter().enumerate()`, which would otherwise be `Array.from(set).entries()`
and is now `$enumerate($iter(..))`. A chain or a zip, whose runtime form
sums its parts' hints, is marked by its counted parts, `counted_parts`,
the rule a `Vec` collected where its type is known already follows. It
can't be marked whole, because a `take(n)` of a B-tree set's `iter()`
can't be stepped from its back.

Held by corpus case `capacity_of_generic_collects.rs`, against native Rust
in both builds, and by the generic in-place collect in
`vec-capacity.test.ts`'s #1180 test. The metamorphic net's seven
`generic-instances` rewrites listed under #1263 are taken off. Mutations:
`collect-vec-by-next-capacity-untracked`, `collect-vec-in-place-capacity-kept`,
`collect-vec-untracked-by-caller`, `given-hint-lost-unmarked`,
`given-chain-marked-whole` and `given-hint-marked-of-any-source` are new,
`chain-part-sources-unmarked` is refitted to `counted_parts`, and `collect-vec-by-next-unhinted` and
`collect-vec-in-place-rest-first` are refitted.

## Amended (2026-11-11, #1316): a `Vec` grown by what `size_hint()` knows

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results;
refusals lifted. In a crate that reads a `Vec`'s capacity, a `Vec` grown by
an iterator whose JS hint is std's was still refused, "whose lower bound
JS's doesn't have". Examples: a `take(n)` of `successors` or of a
`cycle()`, a `rev()` or a `take(n)` of a `flat_map`, an `inspect`, a
string's `chars()` and `char_indices()`. The neighbour shuffle found it,
where a neighbour that reads a capacity made the crate keep them.

`refuse_unkept_lower_bound` now also asks `adapted_hint_known`, the one
answer of whether `size_hint()` of an iterator made in place is std's. Std
grows a `Vec` by that same hint, so where it's std's, so is the growth.

A string's `chars()` is the runtime's `$chars`, whose hint is std's, where
the crate asks it more than its front's steps (#1186). A crate that reads
capacities now counts a `Vec` grown by what holds one as asking it
(`chars_asked.rs`), through `vec_grown_by`, the one answer of which
iterator a `collect()` or an `extend` grows a `Vec` by, which
`hint_asked.rs` now asks too. `lower_bound_is_stds_where` and
`lower_bound_kept_by_parts_where` (`lengths.rs`) take such a `chars()` as
std's, itself or as a chain's or a zip's part. So do a std source whose
form keeps std's hint, `repeat`'s `$repeating` and the like
(`source_hint_kept`), since the runtime's chain and zip sum their parts'
hints. An adapter of `$chars` isn't std's: JS's own `drop(n)` keeps no
hint, so `chars().skip(n)` stays refused, which test/diagnostics.test.ts
now holds.

Held by corpus case `vec_grown_by_iterators_whose_hint_is_stds.rs`, against
native Rust in both builds. The neighbour pairs listed under #1316 run as
native Rust does, and are taken off test/neighbours-known.txt but one.
That one is a zip of an `inspect` collected in the plain build, which
fails on its exact length, not its hint, and is now #1394's. Mutations: 5
new, 2 refitted.
