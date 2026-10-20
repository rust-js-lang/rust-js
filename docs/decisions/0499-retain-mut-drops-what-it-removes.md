# 0499. `retain_mut` drops what it removes

Status: Accepted. Extends [0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`retain` of a `Vec` of values with a destructor drops each it removes,
`$retain` given the items' drop (ADR 0098); `retain_mut`, the same but for
a `&mut` to each, was refused.

## Decision

**`retain_mut` drops each item it doesn't keep as it goes, `$retainMut`
given the items' drop, as `retain` does, of a `Vec` and a `VecDeque`.**

## Why

- **It's std's**: `retain_mut_drops` changes and keeps some items with a
  destructor, of a `Vec` and a `VecDeque`, and numbers through handles, with
  native Rust.

## Consequences

- **`splice`, and `VecDeque::from` of a `Vec`, of values with a
  destructor**, are refused yet. (Amended: neither is, ADRs 0500, 0501.)
