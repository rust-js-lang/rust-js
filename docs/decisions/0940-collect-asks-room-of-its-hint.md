# 0940. `collect()` into a `Vec` asks the room its iterator's hint says, as std's `SpecFromIter` does

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0774](0774-room-past-isize-max-bytes-panics.md).

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
