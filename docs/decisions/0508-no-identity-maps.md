# 0508. No `.map((item) => item)`

Status: Accepted. Extends [0020](0020-structs-and-tuples.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the program can't
tell; the output reads as written by hand.

## Context

Two forms wrote `.map((item) => item)`, which no one writes by hand:

- **`{:?}` of a list of what's shown as itself**, a unit enum's name:
  `` `[${lights.map((item) => item).join(", ")}]` ``.
- **`cloned()` of what a clone of is the value**, a `String`'s:
  `Array.from(calc.vars.keys()).map((item) => item)`. A `Copy` type's
  `cloned()` was already the items themselves; a string's wasn't, though
  its clone is the same string, `needs_clone`'s answer.

## Decision

**A list's `{:?}` of items each shown as itself is `items.join(", ")`; and
`cloned()` of items a clone of can't be told from is the items, as
`copied()` and a `Copy` type's `cloned()` are, by `needs_clone`, the one
answer to whether a clone is more than the value.**

## Why

- **It's what a person writes**: `` `[${lights.join(", ")}]` ``,
  `Array.from(calc.vars.keys())`.
- **It's safe**: a string can't change in place, so the same string is its
  clone; a collected `Vec` is still a new array (`v.slice()`).

## Consequences

- `enum_order`, `calculator` and four more cases, and the playground's
  `programs.js`, lose the map.
- `sources.extend(modules.iter().cloned())` is
  `$extend(sources, Array.from($iter(modules)))`, where
  `sources.push(...modules)` would read better: `extend`'s gap, not this
  one's.
- Mutations map each item to itself again, and clone a string's items.
