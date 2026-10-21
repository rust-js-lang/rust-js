# 0537. A `Cell` gives an `Option` back as it's kept

Status: Accepted. Extends [0051](0051-generic-options.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error compiles as Rust's does.

## Context

An `Option` of what could look like `None`, `Option<Option<u8>>` or a type
parameter's `Option<I>`, boxes its `Some`, `$some(undefined)` for
`Some(None)` (ADR 0051), and a std call that gives one is refused unless
it's known to give it as it's kept. A `Cell` keeps it so, but its
`take()`, `replace()` and `get()` weren't on that list, so itertools'
`Format::clone`, which takes its `Cell<Option<I>>`'s iterator and puts it
back, was refused.

## Decision

**A `Cell`'s `take()`, `replace()` and `get()` give what it held as it's
kept, its `Some` boxed already**, as `Option::take` does: the list in
`refuses_boxed_option` has them.

## Why

`cell_of_a_boxed_option` takes, clones and puts back a generic
`Cell<Option<I>>` of a `Vec` and of an `Option<u8>`, and gets, replaces and
takes a `Cell<Option<Option<u8>>>`'s `Some(Some(1))`, `Some(None)` and
`None`, against native Rust.

## Consequences

- itertools' `Format` and `FormatWith` clone.
- Mutations refuse each of the three.
