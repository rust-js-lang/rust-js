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

## Amendment: a flatten of `Option`s counts as std's `Flatten` does (#2197)

A flatten of `Option`s made in place, or a `flat_map` whose function returns
one, is a filter of the `Some`s. A filter of a `$Steps`, a crate iterator
made knowing its hint, has its own `count`, std's `Filter::count`, which
asks its source's `size_hint` first (filter.rs:141). Std's `Flatten::count`
is `FlattenCompat`'s fold of its inners' counts, and asks no hint. The
count consumer chose `$count` by the type, a flatten that writes `count`,
and so ran the filter's: where the crate asked its iterator's hint anywhere,
here a `zip`'s `len()`, a crate-wide fact that makes each one knowing, the
crate's own `size_hint` was asked once more than native Rust asks it.

The count consumer asks recognition's `counts_by_its_own`, which asks
`flattened_options`, the one owner of that form: the `Option`s it flattens
have no own `count` to reach, so it's counted by its fold, `.reduce`, as a
`filter_map`'s is. Generic code given one counts it by `$count`, the form's
own, as it can't know the type: the form itself must answer `Flatten`'s
and `FilterMap`'s `count`, #2199 (its metamorphic `generic-instances`
rewrite listed there). Case R of ADR 0262 (which of
the crate's steps std calls is observable). Held by
`test/corpus/flatten_of_options_counted_asks_no_hint.rs` and the mutation
`flatten-of-options-counted-as-its-filter`; the conformance table's
`own Every | flatten option | count | none` is off its known list.

The rule that this is one instance of, a runtime form standing for a std
adapter it's not named for, with consumers re-deciding from the type which
of the form's steps to call, is Change A's step 3 (#2112).
