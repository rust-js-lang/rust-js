# 0496. `and` drops what it gives up

Status: Accepted. Extends [0490](0490-a-value-std-leaves-unused-is-dropped-there.md)
and [0179](0179-option-filter-map-or-drops.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`a.and(b)` of an `Option` gives back `b` where `a` is `Some`, dropping
`a`'s value, and `None` where it isn't, dropping `b`; a `Result`'s, `b`
where it's `Ok`, dropping the `Ok`'s value, and its `Err` where it isn't,
dropping `b`. Of values with a destructor, each was refused.

## Decision

**`and` drops what it gives up: the argument it leaves unused, by the one
table of them (ADR 0490), and its own value, by `kept_or`, as `or` drops
an `Err`'s (ADR 0179).** A type parameter's by its given drop, as the drop
of a value std leaves unused, of an `Option<U>` too, is any value's whose
type parameters' drops are given, not a closure's alone.

- **One form for `and` and `Result::or`**, `kept_or`'s, with a destructor
  or not: `some == null ? undefined : "x"`.
- **`xor`**, which drops both where both are `Some`, is refused yet.

## Why

- **It's std's**: `and_value_drops` gives `and` values with a destructor
  of each variant, of an `Option` and a `Result`, a generic one's too, with
  native Rust.
