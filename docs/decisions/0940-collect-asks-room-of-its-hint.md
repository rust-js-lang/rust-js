# 0940. `collect()` into a `Vec` asks the room its iterator's hint says, as std's `SpecFromIter` does

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0774](0774-room-past-isize-max-bytes-panics.md). Amended: what has no
end once it gives an item (below).

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
