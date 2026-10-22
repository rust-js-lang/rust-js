# 0515. `map_while`, and the `Some`s an iterator keeps

Status: Accepted. Extends [0128](0128-iterator-sources.md) and
[0051](0051-generic-options.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same values.

## Context

`map_while` was refused: rust-js had no form for it. And `filter_map` of
a function whose `Some` holds what looks like `None`, `Some(None)`'s, or a
type parameter's, kept the `$some` box the function made, so a
`Vec<Option<u8>>` it collected showed `Some([object Object])`; and
`flat_map` of `Option`s, `filter_map`'s form, too.

## Decision

**`map_while(f)` is std's `map(f)` then its `Some`s till the first `None`,
`take_while`'s: `$lazyTakeWhile(it.map(f), present)`**, holding its source
as `take_while` does, closed to drop what it holds.

**What `filter_map`, `map_while` and `flat_map` of `Option`s keep is taken
out of its box, `.map($someValue)`, where it may look like `None`**: the
items are what the `Some`s hold. `find_map`'s `Option` keeps the box, as
its `Some` is one.

## Why

- **It's the same program**: `map_while_drops` maps numbers while they
  convert, `Option`s, text and values with a destructor, and keeps
  `Some(None)`s by each, against native Rust.

## Consequences

- `dyn_any_generic`'s `filter_map` of a type parameter takes its items out
  of their boxes.
- Mutations refuse `map_while`, hold no source, and keep the boxes.

## Amendment: `flatten` of `Option`s takes each `Some` out of its box

`flatten` of `Option<Option<T>>` items, or of `Option<()>`'s, dropped the
`None`s but kept each `Some(None)` boxed, `$some(undefined)`, which showed
as `Some([object Object])` and wasn't `None` (#145). It takes each out of
its box as `filter_map` does, `.map($someValue)`, through a reference too.
Held by `test/corpus/flatten_boxed_options.rs`.
