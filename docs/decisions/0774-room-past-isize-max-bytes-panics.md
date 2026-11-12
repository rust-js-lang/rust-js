# 0774. Room asked past `isize::MAX` bytes is a `capacity overflow` panic

Status: Accepted. Amends [0315](0315-vec-capacity-and-edits.md), whose
`with_capacity` was `[]` and `reserve` nothing, and
[0322](0322-text-and-slice-names.md), whose `String`'s were too; the
capacity [0419](0419-a-vecs-capacity-kept-where-asked.md) keeps is
checked as well (#183). Amended: a string's `repeat` is JS's own,
unchecked, as the user decided; and room for a length that's asked,
`resize`'s, a range's `collect()`'s and a hash table's, is checked
(#190, below). Amended (2026-10-26): a signed count of text that isn't
empty is given as it is; and a count a `const` gives that fits isn't
checked.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, Rust's, paid for in
output, `$roomFor(n, 4)`; but of items whose size a type parameter gives,
of a string's `repeat`, of growth std's own policy picks and of a
`collect()` of a collection's smaller items, B, listed in the guide's
differences, as the user decided.

## Context

std panics with `capacity overflow` where room it's asked would be more
than `isize::MAX` bytes, as `Vec::with_capacity`, `reserve`,
`String::with_capacity`, `vec![x; n]` and `repeat` document. rust-js's
`usize` is 32-bit ([0011](0011-numbers.md)), so that's any request over
2³¹ − 1 bytes. The JS did something else: `with_capacity` was `[]` and
`reserve` nothing, so no panic; `vec![0u8; usize::MAX]` was `new
Array(4294967295).fill(0)`, which runs out of time or memory; `"ab".repeat(n)`
threw JS's `RangeError: Invalid string length`, not the panic, and only
where the engine's limit is.

## Decision

**Where a capacity is asked, the room is checked as std checks it:
`$roomFor(count, size)` panics `capacity overflow` where `count` items of
`size` bytes are more than `isize::MAX` bytes, or more than `usize::MAX`
items, of no size too.** The size is rustc's for wasm32, `size_of`'s
([0562](0562-a-pointer-is-a-place-and-an-address.md)); the lowering's
`room_for` is the one owner of the check, and `$roomFor` the runtime's.

| Rust | JS |
|---|---|
| `Vec::<u32>::with_capacity(n)`, a `VecDeque`'s, a `BinaryHeap`'s | `$roomFor(n, 4); []` |
| `v.reserve(n)`, `reserve_exact(n)` | `$roomFor(v.length + n, 4)` |
| `String::with_capacity(n)` | `$roomFor(n, 1); ""` |
| `s.reserve(n)`, `reserve_exact(n)` of a `String` | `$textRoomFor(s, n)`, its bytes counted only where they may not fit |
| `vec![x; n]` | `$roomFor(n, 4)` first, after `x` is made; `$repeat(x, n, 4, clone, drop)` of one with a destructor, which drops `x` where there's no room, as std's unwinding does |
| `"ab".repeat(n)` | `"ab".repeat(n)`, JS's own: the engine refuses a string past its length first (amended, below) |
| `[a, b].repeat(n)` | `$repeatItems(v, n, 4)`, room for `v.length * n` |

A check that can't fail is left out: room for none more, `with_capacity(0)`
or `reserve(0)`; a count known to fit, `Vec::<u64>::with_capacity(10)`;
a count alone of items of no size, `Vec::<()>::with_capacity(n)`. Room
for more of items of no size is checked by their count, `len + n` past
`usize::MAX`, as std's `checked_add` panics; so is a slice's `repeat`,
whose `checked_mul` does.

What's given to the call runs first, as Rust runs it: `vec![made(); n]`
is `const item = made(); const n = pastHalf(); $roomFor(n, 1);`.

**Of items whose size a type parameter gives**, `fn f<T>(n: usize) ->
Vec<T> { Vec::with_capacity(n) }`, the size isn't known where the call is,
and giving it would add a `TSize` to each generic function that asks room
and refuse a trait's method or a library's function, which can't be given
one ([0145](0145-type-facts.md)). So their size is taken as none:
room for them is checked by their count alone. That's case B, decided by
the user: with_capacity makes no array in JS, so nothing runs out, and a
`vec![x; n]` that std would refuse runs out of memory instead.

## Consequences

- `Vec::with_capacity(n)` of a count not known here is a statement more,
  `$roomFor(n, 4);`, and `reserve(n)` a statement where it was none.
- The capacity std's growth picks isn't kept (0419): a `push` past the
  room std's doubling asks, which may be more than its length, isn't
  checked, nor where a `Vec` grows by itself, `extend`, `resize`,
  `collect()`, nor a `HashMap`'s or a `HashSet`'s room; filed as their
  own question (#190), and amended: a length that's asked is checked
  (below).
- A JS string can't be as long as std's: `"-".repeat(1 << 30)`, a
  gigabyte, throws JS's `RangeError` where the engine's limit is less,
  Node's about 2²⁹ UTF-16 units; that's the engine running out, as memory
  does. Of a string's `repeat`, that's all there is (amended, below).
- Tested by `capacity_overflow.rs`, against native Rust, whose `usize`
  there is 64-bit, by room past half of `usize::MAX`, which is past both
  `isize::MAX`s; and by `runtime-package.test.ts`, where rust-js's 32-bit
  `isize::MAX` ends, one item either side.

## Amendment: a string's `repeat` is JS's own

`"ab".repeat(n)` printed `$roomFor(n, 2); "ab".repeat(n)`, and React's
ReactPerformanceTrackProperties, whose `prefix + '\xa0\xa0'.repeat(indent)`
and `' '.repeat(start)` are JS's own, read `$roomFor(indent, 4);` and
`$roomFor(start, 1);` before each. **A `str`'s or a `String`'s
`repeat(n)` is JS's `repeat`, unchecked**, as the user decided: a JS
string is at most 2³¹ − 1 UTF-16 units ([0682](0682-a-js-strings-length-is-at-most-int32-max.md)),
V8's (Node's, Chrome's) less than 2²⁹, each of 3 bytes at most, so less
than `isize::MAX` bytes. A repeat std refuses, V8 refuses too, before
it's made: a `RangeError: Invalid string length`, where std panics
`capacity overflow`. JavaScriptCore's (Safari's, Bun's) go to 2³¹ − 1
units, so there a string of text not all ASCII, past a billion units, may
be made where std panics, as memory lets it. That's case B, rare and
harmless: a program that asks a string that long runs out either way but
in that corner, and the error differs, which the guide lists among its
differences.

A `String::with_capacity(n)` and a `String`'s `reserve(n)` make no
string, so nothing else would refuse them: they're checked still, as a
`Vec`'s room and a slice's `repeat` are. `room_for` is still the one check of room asked; a string `repeat`
makes isn't asked of it, `str::repeat` being recognized as JS's method.
Tested by `test/lowering-strings.test.ts`, the bare `repeat` and its
`RangeError`; `capacity_overflow.rs` no longer asks a string past it.

## Amendment: room for a length that's asked, and a hash table's (#190)

Where a collection grows to a length it's asked, std checks the room as
`with_capacity` does: `resize(n)` and `resize_with(n, f)` reserve what
`n` needs; `collect()` of an iterator that knows its length asks
`with_capacity` of it first; a `HashMap`'s or a `HashSet`'s
`with_capacity(n)` and `reserve(n)` ask hashbrown's table, which panics
with its own message, `Hash table capacity overflow`. rust-js did none of
them: `v.resize(usize::MAX, 0)` pushed until memory ran out, and
`(0..usize::MAX).map(f).collect()` called `f` four billion times, where
std panics before it calls `f` once. **They're checked as std checks
them**, as the user decided (2026-10-25): case C, Rust's.

| Rust | JS |
|---|---|
| `v.resize(n, x)` | `$resize(v, n, x, 4)`, room for `n` asked first, `x` dropped where there's none, as std's unwinding drops it |
| `v.resize_with(n, f)` | `$resizeWith(v, n, f, 4)` |
| `(a..b).map(f).collect()`, `(a..=b)`, `.rev()` | `$range(a, b, 4).map(f)`, room for `b - a` asked first |
| `HashMap::<K, V>::with_capacity(n)`, a `HashSet`'s | `$tableRoomFor(n, size, align); new Map()`, of a `(K, V)`'s size and alignment, a set's `T`'s |
| `m.reserve(n)` of a `HashMap` or a `HashSet` | `$tableRoomFor(m.size + n, size, align)` |
| indexmap's `IndexMap::with_capacity(n)`, `reserve(n)` | `$tableRoomFor(n, 4, 4); $roomFor(n, size)`, its table of each entry's place, then its `Vec` of entries, each its hash, its key and its value |

`$tableRoomFor` is hashbrown 0.17's sizing for wasm32, whose groups are 8
control bytes: 4, 8 or 16 buckets of fewer than 15 entries, else the
power of two past 8/7 of them, a `usize`'s eighth at most; a control byte
each and a group's more, after the entries, aligned to the entry's
alignment or the group's, past `isize::MAX` bytes less that alignment's
padding a panic. Of entries of a byte or none, hashbrown takes 8 buckets
at least where it's asked fewer than 8, which fit as surely as 4: left
out. A size that's left out where room surely fits, as `room_for`'s: a
`resize` or a range of a count known to fit; a table of a count whose
bound, fewer than 3 times as many buckets and 16, fits.

`room_for` stays the one check of room asked; `room_fits` is its one
answer of what surely fits, which `room_size_of` gives the helpers that
check it themselves, `$resize`'s and `$range`'s, and `table_room_for` is
its hash tables' check, of the same `isize::MAX`.

**What's left JS's**, listed among the guide's differences (case B), as
the user decided (2026-10-25):

- Growth std's own policy asks, which rust-js doesn't model (0419): a
  `push`, `extend` or `insert` past the capacity std's doubling picks.
  `Vec<Option<[u8; 1 << 20]>>`'s 1,025th `None` panics in Rust, 2,048
  MiB, and is an `undefined` more in JS.
- A `collect()` whose length is a collection's that holds smaller items,
  `ids.iter().map(|x| x.to_string())`, whose `String`s are bigger than
  its `u32`s: past `isize::MAX` bytes of them only of 179 million ids, or
  of a few thousand items each a mebibyte made of little, such a `None`.
  Checking it would put `$roomFor(ids.length, 12);` before each such
  `collect()`, among the commonest code there is, for what no program
  asks.

And one left JS's for now, filed (#246), which the user's decision covers
but this amendment doesn't reach: a count collected through another
adapter, `(0..n).enumerate()`, `step_by`, `zip`, or a lazy chain,
`repeat_n(x, n)`, `repeat(x).take(n)`, or a range of `u64`s. Their length
isn't the range's, or the JS iterator that steps them doesn't know it,
so the collect has no count to check; listed among the guide's
differences until it's fixed.

Tested by `asked_length_overflow.rs`, against native Rust, of room past
both `isize::MAX`s and room that fits, and an item dropped as `resize`
unwinds; by `runtime-package.test.ts`, where native wasm32's ends, a
mebibyte item either side, and a table of each entry's size where native
wasm32 hashbrown's `try_reserve` first says `CapacityOverflow`, and at
the byte where a table of 4 or 16 buckets ends; and by `bound-maps`, of
hashbrown's and indexmap's own maps. The Consequences' growth (#190) is
this amendment's.

## Amendment: a signed count of `repeat` is given as it is (2026-10-26)

React's hydration diff indents by an `i32` that may go below 0, which
its port gives `repeat` as a `usize`:

```rust
"  ".to_owned() + &"  ".repeat(indent as usize)
```

That read `"  ".repeat(indent >>> 0)`, where upstream's is
`'  '.repeat(indent)`. **The cast of a signed integer of 32 bits or fewer
to `usize` that's the count of a `repeat` of text written there that isn't
empty is the integer as it is** (`repeat_counts`, a kept cast): a negative
`i32` is `2^31` or more as a `usize`, so its repeat is `2^31` bytes or
more, past `isize::MAX`, which std panics `capacity overflow` of; JS's
`repeat` throws a `RangeError` of a negative count, as it does of one
that large, in every engine, a JS string being at most `2^31 - 1` units.
So each throws what it threw, the engine's `RangeError` where std panics,
this decision's difference already, in V8 and JavaScriptCore alike;
only its message, which this decision already leaves to the engine,
says `Invalid count value: -1` where it said `Invalid string length`.

- **Of empty text it's cast as before**: `"".repeat(-1 as usize)` is `""`
  in Rust, `"".repeat(4294967295)` in JS, where `"".repeat(-1)` throws.
- **Of text the body doesn't write**, a variable's, which may be empty,
  as before.

Case B, this decision's own, as before: no new difference. Held by
`test/corpus/ptr_eq_items_and_counts.rs`, the port's `indentation`,
`"".repeat(indent as usize)` of `-1`, `""`, and `"  "`'s a panic, run
against native Rust, and the mutation `repeat-count-of-empty-text`.

## Amendment: a count a `const` gives is its value (2026-10-26)

A count that's a `const` the crate reads by its name, React's
`vec![None; TOTAL_LANES]` of `TOTAL_LANES = 31`, was checked,
`$roomFor(TOTAL_LANES, 12)`, though rustc knows it's 31, which fits: the
check reads the JS it's given, a name, and only a literal's value told
it that it fits. Found porting React's `createLaneMap`.

**`vec![x; n]`'s count is what rustc evaluates where it's a constant**:
the call's MIR argument, read where the call is chosen
(`mir_vec_call`), and `room_for_count` leaves out a check that can't
fail by `room_fits`, as of a literal. One that doesn't fit is still
checked by its name, `$roomFor(ALL, 4)` of `ALL = usize::MAX`, and
panics as std does. Case C still: nothing observable changes, as the
check left out is one that can't fail.

Held by the corpus's `capacity_overflow.rs`, against native Rust, and
`scripts/mutations/lower/std_types/vec.ts`'s
`room-of-a-constant-count-checked` and
`room-of-a-constant-count-unchecked`.

## Amendment: a count collected through another adapter (#246)

`repeat_n(1u8, usize::MAX).collect::<Vec<_>>()` stepped until memory ran
out, where std's `SpecFromIter` asks `with_capacity(n)` of a `TrustedLen`
first and panics `capacity overflow`. So did `repeat(x).take(n)` and a
range's `step_by(s)`. A program could catch std's panic with
`catch_unwind`, and the JS ran on, so results change: ADR 0262 has Rust's
behavior kept.

`collected_counted` (`mir/iter/collect.rs`) asks `room_for`'s `$roomFor(n,
size)` of a `Vec`'s `collect()`, where three things hold:
- the iterator is made in place;
- its length is a count the program gives (`repeat_n`, a `take(n)` of an
  endless one, a range's `step_by`), counted by `len_in_place` (ADR 0986);
- nothing else asks room for it, as a range's `$range(a, b, size)` or a
  struct's `$roomForHint` does.

It's asked before the collect steps it, of a `TrustedLen` one, so a `map`'s
closure never runs. Of another, whose first step std takes before it asks,
it's asked only where stepping runs no code. What makes the iterator, where
that runs code, `repeat(made()).take(n)`, is held first, so it runs before
the panic as Rust's does. An array's items that are there already aren't
counted: their room is the array's.

`len_in_place` counts the three forms, which also lifts the refusal of
`len()` of a range's `step_by`. A count of numbers known folds to the
number, `(0..10).step_by(3)` being 4, so `room_fits` leaves its check out.
`$repeatingN` hints what it has left and `$repeating` `(usize::MAX, None)`,
so std's struct of a `map` of one asks its room by `$roomForHint` too.

The guide's difference 26 no longer lists these forms.

- `test/corpus/collected_counts_ask_room.rs` checks each form against
  native Rust, in both builds. It's in the forms test's `few` list.
- The mutations are `counted-collect-room-unasked`,
  `counted-collect-made-after-room`, `repeat-n-uncounted`,
  `step-by-uncounted`, `take-of-endless-uncounted`,
  `repeating-n-unhinted`, `range-count-unfolded` and
  `step-by-count-unfolded`.

## Amendment: a range from 0 mapped into a `Vec` is `Array.from` (#361)

`(0..queens_count).map(|_| None).collect()` was `$range(0, queensCount,
12).map(() => undefined)`, an array of its indices made only to be mapped
away. **`(0..n).map(f)` collected into a `Vec`, of one arrow `f`, is
`Array.from({ length: n }, (_, i) => ..)`**, its room asked first,
`$roomFor(n, size)`, where `$range` asked it, so it panics capacity
overflow before `f` runs, as std's `collect` does
(`sources::counted_range`, from `collected_counted`, which has the
statements). Each item is its own `f`'s, a fresh `[]` each. Not of a range
from another start, an inclusive one, a reversed one, more than one map,
nor into a heap, whose `$heapFrom` is made of the array: `$range` as
before. Case N: the same items in the same order, the same check first.
Held by `test/corpus/range_map_collected.rs` against native Rust, 13
other corpus cases reading so, and the mutations `counted-range-*`
(`mir/iter/sources.ts`, `mir/iter/collect.ts`).

## Amendment: a count read through a call asks its room by the hint (#1044)

`std::iter::repeat_n(1u8, *all.as_ref().unwrap()).collect::<Vec<_>>()` of
`usize::MAX` threw JS's `RangeError`, or ran on, where native Rust panics
`capacity overflow`. `len_in_place` reads a count only where reading it
again runs nothing, so of `$unwrap(all)` it had no count, and
`collected_counted` asked no room. The same of `repeat(x).take(n)`'s and
a `step_by(s)`'s count read through a call; of a `repeat_n` of a value it
clones or drops, `$repeatingN(x, n, clone, drop)`, whose count it never
read; and of an iterator that drops what it holds, `repeat_n(Dropped(1),
n)`, whose `$roomFor(n, size)` in place threw without closing it, so the
value wasn't dropped as Rust's unwinding drops it.

**`counted_in_place` says why it has no count: `Uncounted::RunsCode` where
a count it reads runs code where it's read, after what it's made of is
counted, else `Uncounted::Unknown`; `len_in_place` is its answer as an
`Option`.** Of a count that runs code, of one it would hold part of the
iterator to read, and of an iterator that drops what it holds,
`collected_counted` asks the room its hint asks, `$roomForHint(it, size,
trusted)` (ADR 0940's `grown_by_hint`), as std's `SpecFromIter` does, the
count read once where the iterator is made; the hint ask closes the
iterator where it throws. It's the iterator itself, marked as it is, so
it's collected as one made in place still. Not where what it's collected
into asks its hint already (`grown_ask`), of one whose hint may run the
crate's own, `repeat(1).chain(own).take(counted(n))`'s, asked once. A
count it reads of an iterator that drops nothing is `$roomFor(n, size)` as
before; one known to fit, nothing. Case N for each: the same panic at the
same place, or the same items. No other corpus case's JS changes.

Only of adapters made in place of std's `repeat`s and ranges, whose JS
hints are std's; an iterator held in a local, #1046's, below.

- `test/corpus/collected_counts_ask_room.rs` checks counts held and read
  through a call, a dropped value repeated, and a crate's own hint asked
  once, against native Rust.
- The mutations are `counted-collect-of-a-called-count-unasked`,
  `counted-collect-asked-by-hint-unmarked`,
  `counted-collect-of-what-drops-asked-in-place`,
  `counted-collect-asked-by-hint-twice` (`mir/iter/collect.ts`),
  `count-read-through-a-call-read-again` and
  `repeat-n-of-a-cloned-value-uncounted` (`mir/iter/sources.ts`).

## Amendment: a counted iterator held before it's collected asks its hint (#1046)

`let it = (0..n).rev().step_by(2); it.collect::<Vec<_>>()` of `usize::MAX`
threw JS's `RangeError` where native Rust panics `capacity overflow`, and
`repeat_n(Dropped(1), *n.as_ref().unwrap())` too, which MIR holds in a
local as `Dropped(1)` is made before the `unwrap`'s check, as it does under
a `RefCell` guard's `finally`: a local is no form `counted_in_place`
reads (`Uncounted::Unknown`), so no room was asked. Asking its hint
instead needs each form's hint to be std's, and two weren't: the
generator `$lazyRangeBack`, a reversed range's `step_by`, hinted nothing,
and a `step_by` as a filter of every `n`th, `.filter((_, i) => i % n ===
0)`, a filter's `(0, upper)`, not std's `StepBy`'s.

**Each runtime form hints as std's, the one authority of what it is; so a
counted collect of `Uncounted::Unknown` asks the room its hint asks,
`$roomForHint`, as one of `Uncounted::RunsCode` does.** `$lazyRangeBack`
is a `$Steps` whose hint is std's `StepBy`'s of the reversed range, of `n`
left from what it gives next, `1 + (n - 1) / step`, which is std's before
its first item and after, of numbers, BigInts and `char`s, `$stepsLeft`'s
past `usize::MAX`. A `step_by` of what no array's items bound
(`bounded_by_items`) is `$stepBy(it, n)`, whose hint is `StepBy`'s; of an
array's, the filter still, whose items bound it. No catalog of which
forms hint as std's: a form that didn't would be a bug of that form's.
Case N: the same panic at the same place, or the same items. Eight corpus
cases' JS changed: a collect of a `step_by` of the crate's own iterator, of
a range from a function, a `RangeFrom`'s, a `u64` range's reversed, asks
`$roomForHint`, and two `step_by`s of what no array bounds are `$stepBy`,
`$cycle(a).drop(2)`'s among them.

- `test/corpus/counted_iterators_held_ask_room.rs` checks held forms and
  ones a guard or an `unwrap`'s check holds, of `usize::MAX` and of five,
  and `$lazyRangeBack`'s hint given to generic code, against native Rust.
  The `option` and `rc-refcell` rewrites of `collected_counts_ask_room.rs`
  run as native, taken off.
- The mutations are `counted-collect-held-unasked` (`mir/iter/collect.ts`),
  `step-by-of-unbounded-as-filter` (`combinators/adapters.ts`),
  `lazy-range-back-unhinted` and `lazy-range-back-hint-counted-apart`
  (`runtime/lazy_range.ts`).

## Amendment: a type parameter's items' room is checked by its caller's size (2026-11-12, #1432)

Case B for items whose size a type parameter gives is reopened by the
user: results change, a `capacity overflow` panic std makes and JS didn't,
so Rust's behaviour is kept (ADR 0262's third case). The reason it was B
is mostly gone: a library's function is given facts by its consumers, and
a trait's associated type's by its dictionaries (ADR 0145, #990, #1312).

- **Each call that checks room by the bytes of its first type argument's
  items** (`recognition::checks_room_of_first`: a `Vec`'s, a `VecDeque`'s
  or a `BinaryHeap`'s `with_capacity`, `reserve` and `reserve_exact`,
  `vec![x; n]`, a `Vec`'s `resize` and `resize_with`, a slice's `repeat`)
  asks its caller their size in any crate, `TypeFact::Room` (ADR 0419's
  #1260 amendment), named `TSize`, and `room_item_size` reads it
  (`room_value`): `fn f<T>(n: usize) { Vec::<T>::with_capacity(n); }` is
  `$roomFor(n, TSize)`. Room for one item fits whatever its size, as no
  type is more than `isize::MAX` bytes, so it isn't checked.
- **Left by their count alone**, the guide's difference 23: an impl's
  associated type its dictionary doesn't carry (#934), `I::Item` in an
  impl's method, `$roomFor(n)`, as `Room` is given `undefined` there (a
  built type of it, `Option<I::Item>`, is given by the impl's dictionary's
  maker, `wrapperRoom(16)`); a collect's, whose room
  std asks as its hint says, which the lowering decides as it collects,
  so asking `Room` of each gave generic functions a `TSize` they never
  read (#1436); and a hash table's entries, which need their alignment
  too (#1437). Each is filed.

Tests: corpus case `capacity_overflow_of_generic_items` (each of those
calls in generic code of `[u64; 4]`s, past `isize::MAX` bytes on wasm32 and
native both, and within it, and of no size), against native Rust in each
build; corpus case `room_of_an_impls_associated_items`, an impl's method's
and a copied default's room of `I::Item` and `Option<I::Item>`; 2 corpus
snapshots changed, a generic function given its `TSize`; and a copied
default's `reserve` of `Vec<Option<Self::Item>>`, std's capacity 4. A
`reserve` of items whose size isn't given grows to a capacity not known,
never as if they had none.

## Amendment: a generic collect counted by the program asks its items' size (2026-11-12, #1436)

A generic `collect()` into a `Vec` of a type parameter's items checked its
room by their count alone, #1432's last case. Std's `SpecFromIter` reserves
a `TrustedLen`'s upper bound first, so a count past `isize::MAX` bytes
panics `capacity overflow` (ADR 0262's third case).

- **One answer of which collects check room upfront,
  `Recognition::counted_by_program`**: a `TrustedLen` whose count is the
  program's, a range's, a `repeat_n`'s, a `take(n)`'s of an endless one or
  of one counted so, a range's `step_by`, through adapters that keep it
  (`length_kept_source`), or a chain or a zip of such. Not of a
  collection's items, whose room is the guide's other difference (a
  collect whose length is a collection's), nor a `filter`'s, nor into a
  `Result` through std's `GenericShunt`, whose lower bound is none.
- **The analysis asks `Room` of these alone** (`room_asked`), and **the
  lowering reads it at each**: where nothing else checks its room, a zip's,
  a chain's, a held map's or a map of a range `counted_range` doesn't take,
  its collect asks the room its hint asks, `$roomForHint(it, TSize, true)`.
  So no generic function takes a `TSize` it doesn't read, which
  test/architecture-generic-facts.test.ts holds of every corpus snapshot
  (#1443 lists six older ones).
- **Of items of a type known here**, the same shapes, a zip's, a chain's, a
  held map's of a range, a `take(n)` of a range, checked no room; amended
  below (#1441).

Tests: corpus case `collect_room_of_generic_items` (each shape past
`isize::MAX` bytes on wasm32 and native both and within it, of items with
a destructor too, and a collection's items', a filter's and a `Result`'s
taking no `TSize`), against native Rust in each build; vec-capacity.test.ts's
signatures of a counted, a collection's and a filter's collect; 2 corpus
snapshots changed, a generic collect of a map of a range given its
`TSize`. Mutations `counted-by-program-of-anything`,
`counted-by-program-not-of-zip`, `counted-by-program-not-of-a-finite-take`,
`collect-counted-by-program-unroomed`, `collect-room-left-to-any-mapped-range`
and `collect-room-unasked`, caught.

## Amendment: a generic hash table's room is checked by its entries' layout (2026-11-12, #1437)

Room hashbrown checks for a table of entries whose layout a type parameter
gives was checked by their count alone (`table_room_for`'s "of an entry a
type parameter's size makes, of its count alone"), #1432's last case:
`HashMap::<K, V>::with_capacity(n)` in generic code didn't panic `Hash
table capacity overflow` where native Rust does (ADR 0262's third case).

- **The entry's layout has one owner, `entry_layout`**: rustc's where it
  lays it out here, else its caller's size and alignment, `KVSize` and
  `KVAlign` of a `(K, V)`, a set's `TSize` and `TAlign` (a set's entry is
  its item, laid out as `(T, ())` is), or `0` and `1` of #934's case.
  `table_room_for` and `$reserveHint`'s two sites read it.
- **The analysis asks `Room` and `Align` of the entry exactly where the
  lowering checks it** (`table_room_asked`): a `HashMap`'s or a `HashSet`'s
  `with_capacity` or `reserve` (`table_room_entry`; an indexmap's, its
  buckets' size), and a collect into one or an `extend` of one whose
  hint's room is asked first (`room_of`'s entries and `hint_room_seen`,
  moved to `Recognition` to be the one answer of both). A count of none
  fits without a check; any other may not, of an entry of any size.

Tests: corpus case `table_room_of_generic_entries` (each of a map's and a
set's, past `isize::MAX` bytes of its table on wasm32 and native both and
within it), against native Rust in each build; one corpus snapshot
changed, `map_entry_hashes_its_key_once`'s generic tables given their
layouts. Mutations `table-room-of-capacity-unasked`,
`table-entry-layout-of-a-param-unread`, `table-room-of-a-collect-unasked`
and `table-room-of-an-extend-unasked`, caught; three refitted.

## Amendment: a collect counted by the program checks room where its count may pass it (2026-11-12, #1441)

Of items of a type known here, a collect of a zip's, a chain's, a held
map's of a range or a `take(n)` of a range checked no room, and ran on
making the array where std panics `capacity overflow` (ADR 0262's third
case). It now goes the way a counted collect does (`collected_counted`),
for each `counted_by_program` accepts, items of a type parameter's too,
which folds #1436's fallback into it:

- **Where its count surely fits, nothing**, its collect as it reads,
  `[...$lazyRange(0, 2), ...$lazyRange(5, 7)]`: no more items than its form
  says, read by the iterators' owner (`count_bound`: a range's of literal
  bounds, numbers, `u64`s or `char`s, through a `rev()`, an `enumerate()`, a
  `map(f)`, a lent one, a literal `take(n)`, a chain's sum, a zip's fewer),
  or a count read of literals alone (`folded_count`, `Math.min(8, 3)`), that
  fits (`room_fits`).
- **Where it reads a count that may not fit, `$roomFor(n, size)`** first,
  `counted_in_place`'s, a chain's now the sum of its halves'
  (`chain_parts`).
- **Where reading its count would run code**, a bound's call, or it's held
  where its count can't be read, **the room its hint asks**, `$roomForHint`,
  once it's made: each bound runs once, in Rust's order, before any item
  is made. Of a bound that's a conversion, `Number(BigInt.asUintN(..))`,
  that's this form too, `Array.from($roomForHint($lazyChain(a, b), ..))`.

Tests: corpus case `collect_room_counted_by_the_program` (each shape past
`isize::MAX` bytes on wasm32 and native both, one within it, a literal
chain kept as it reads, and bounds that print, run once in Rust's order,
before any item), against native Rust in each build; vec-capacity.test.ts's
forms of a literal chain, a counted chain and a literal `take(n)`; 22
corpus snapshots changed, each a collect of a count that may pass the
limit given its check. A range's array mapped, `$range(a, b, size)`, and
`counted_range`'s, which ask their room themselves, are left to their
own forms. Mutations `collect-count-of-a-call-read-again`,
`collect-counted-by-program-uncounted`,
`collect-room-checked-where-it-surely-fits`, `chain-uncounted` and
`count-bound-of-a-take-unread`, caught; two refitted, and #1436's
`collect-counted-by-program-unroomed`, whose fallback this replaces,
removed.

## Amendment: a hash table's room through a `FromIterator` bound (2026-11-12, #1450)

A collect into a hash table through a generic `B: FromIterator<A>`, whose
dictionary the caller makes in generic code, `at::<_, HashMap<usize, V>>`,
checked the table's room by its count alone: the dictionary's `from_iter`
(`collecting_entry`) asks the room of its hint, as it does of any type
parameter's iterator, by `entry_layout`, but no caller was asked the
entry's layout, so JS filled the table where hashbrown panics `Hash table
capacity overflow` (ADR 0262's third case). `table_room_asked` now asks
`Room` and `Align` of the entry where a call of a function not std's gives
a `FromIterator` bound a hash table's dictionary, the same entries
`room_of` names; std's own `collect()` makes no dictionary and is asked
as a collect.

Tests: corpus case `table_room_of_generic_entries`, a collect through a
bound past the limit and within it, and its `generic-instances` rewrite,
off test/metamorphic-known.txt; mutation
`table-room-of-a-from-iterator-dictionary-unasked`, caught.
