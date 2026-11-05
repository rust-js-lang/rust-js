# 0940. `collect()` into a `Vec` asks the room its iterator's hint says, as std's `SpecFromIter` does

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0774](0774-room-past-isize-max-bytes-panics.md). Amended: what has no
end once it gives an item (below); amended: what isn't `TrustedLen` is
stepped once before its hint is asked (below, #724).

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
  `endless-room-by-lower`.

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
the `Vec` is full as it grows. rust-js keeps no native capacity (ADR
0419), so the JS asks it once. And a collect the recognizers make in
place, of the crate's iterator, asks no hint at all. Both are #728's.

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
