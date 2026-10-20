# 0518. A held `filter_map`, and `$closing` a JS iterator

Status: Accepted. Extends [0465](0465-dropping-an-adapter-is-closing-it.md)
and [0515](0515-map-while-and-kept-somes.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what drops would
change, so Rust's is kept.

## Context

A `filter_map` or a `map_while` of items with a destructor, held and
dropped, was refused: neither was among the adapters closed to drop what
they hold, and `filter_map` didn't hold its source closed, so what it
hadn't given was never dropped.

And `$closing` and `$peekable`, which make JS iterators, weren't in
`ITERATOR_HELPERS`: a chain of one was written inside `Iterator.from(..)`,
and one consumed whole wasn't made eager.

## Decision

**`filter_map` holds its source closed, as `map` does, and a held
`FilterMap` or `MapWhile` drops by closing it, `closed_on_drop`'s.**
`$closing` and `$peekable` are in `ITERATOR_HELPERS`.

## Why

- **It's the same program**: `held_filter_map_drops` steps and drops each,
  against native Rust.
- **It reads as written**: `adapter_drops` and `flat_map_drops` lose their
  `Iterator.from` wrappers and the values spilled for them.

## Consequences

- Mutations refuse a held `FilterMap`, leave its source unclosed, and take
  `$closing` for what isn't a JS iterator.
