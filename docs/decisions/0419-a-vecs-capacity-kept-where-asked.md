# 0419. A `Vec`'s capacity is kept where it's asked

Status: Accepted. Amends [0315](0315-vec-capacity-and-edits.md), which
refused `capacity()`. Amended by
[0774](0774-room-past-isize-max-bytes-panics.md): the room asked is checked
first; and below (#849, #1180), its number std's growth's.

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
  them is; and a bound's `extend` of a `Vec<T>` of a type parameter's
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

