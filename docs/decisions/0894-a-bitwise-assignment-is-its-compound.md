# 0894. A bitwise operator's assignment is its compound

Status: Accepted. Extends preparation's compound (`prepare.rs`'s
`compound`), which [0631](0631-a-step-of-one-is-plus-plus.md) prints a
step of one of.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: `t = t op v` and `t op= v` of a path `t` evaluate its object
and its key, read it, evaluate `v`, compute and write, in that order,
the same in both forms. The object of a member path is evaluated once
where it was twice, and a path is variables, properties and constant
keys (`same_path`), which read the same both times.

## Context

React's ReactFiberLane is a bit set's arithmetic: `lanes &= ~lane`,
`root.pendingLanes |= updateLane`, `entangledLanes |= lane`. Its port,
`lanes &= !lane;` and, of a `Cell` field, `root.pending_lanes.set(
root.pending_lanes.get() | lane)`, printed `lanes = lanes & ~lane;` and
`root.pendingLanes = root.pendingLanes | lane;`. Preparation made `t = t
op v` the compound `t op= v` of `+ - * / %` and `??` alone, so a bitwise
operator's and a shift's assignment read as no JS developer writes it.

## Decision

**A statement `t = t op v` of `&`, `|`, `^`, `<<`, `>>` or `>>>`, of a
path `t`, is `t op= v`**, as it is of `+`: `lanes &= ~lane`, `n >>>= 1`
of a `u32`, `root.pendingLanes |= lane` of a `Cell`'s field (ADR 0270's
property), `fiber.flags &= mask` of a field.

Preparation decides it, in the one place it makes every compound; the
printer writes each operator's assignment.

## Why

- **It's how JS writes a bit set**, upstream's spelling, and one form of
  output for `t = t op v` whatever the operator.
- **It computes the same**: JS's `a op= b` is `a = a op b` of the
  reference `a` evaluates to, and both read `a` before `v`.
- **A wrap stays a wrap**: `x = (x << 1) >>> 0` of a `u32` is no compound,
  the value's operator is the wrap's, `>>>` of `x << 1`.

## Alternatives

- **Decided in lowering**, from Rust's `BinOp` assignments: a `Cell`'s
  `set` of its `get` is a call, no assignment in MIR, and the place's
  read and write are made apart by then; preparation sees the one
  statement every form ends as.

## Consequences

- Four corpus programs and a snapshot write their bit sets' assignments
  as compounds: `bitwise_operators.rs`'s `flags[0] |= other[0]` and
  `flags[0] >>>= by & 7`, `integer_bits.rs`'s `c >>>= 1`, a 64-bit
  hash's `hash ^= BigInt(b)` (`integers_128.rs`, `wide`), and
  `generic_bitwise_operators.rs`'s. The guide's numbers page shows
  `lanes &= ~lane` and a `Cell`'s `root.pendingLanes |= lane`.

## What holds it

- `test/lowering-bits.test.ts`, "a bitwise operator's or a shift's
  assignment is its compound", run in JS: `&=`, `|=`, `^=`, `>>=`, a
  `u32`'s `>>>=`, a cell's field and a field.
- Mutations `bitwise-assign-spelled-out` (`prepare.ts`),
  `bitwise-and-assign-printed-or` (`to_oxc/statements.ts`).
