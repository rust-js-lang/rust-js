# 0497. A std trait's dictionary drops its associated type

Status: Accepted. Amends [0492](0492-a-kept-default-steps-its-impls-iterator.md),
extends [0178](0178-associated-type-drops.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

An associated type's value is dropped by its drop: of a trait whose bound
is given a dictionary, the dictionary's entry, `$dropOffset` (ADR 0178);
of one that's given none, an iterator's, a slot its function's given (ADRs
0421, 0489). ADR 0492 kept every std trait's out of dictionaries, to stop
an iterator's being looked for in its `Iterator` dictionary, and so `a + b`
of a `T: Add` whose `Output` has a destructor was refused.

## Decision

**A std trait's associated type is dropped by its dictionary's entry, as a
crate's trait's is, where its bound is given a dictionary: `TAdd.$dropOutput`
of an `Add`'s `Output`; an impl's dictionary has it where its type has a
drop.** An iterator's, of no dictionary, is its slot's, found first, as
ADR 0492 makes it.

```js
function sum_drop(a, b, TAdd) {
  const _x = TAdd.add(a, b);
  TAdd.$dropOutput?.(_x);
```

## Why

- **It's std's**: `operator_output_drops` adds values with a destructor,
  and numbers, generically, with native Rust.
- **One rule**: a bound given a dictionary carries its associated types'
  drops, whoever's trait it is.
