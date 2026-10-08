# 0193. `Some` of a unit variant is tested as the variant

Status: Accepted. Amends [0030](0030-option.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`Some(p)` of an `Option` is `o != null`, and `p`'s test of the value
(ADR 0030), but for a constant or a range, whose test is false of
`undefined` already: `o === 0`, `o >= 1`. A unit variant's test is its
name, `d === "Up"`, which is as false of `undefined`, yet it was written
`d != null && d === "Up"`, as react.dev's `IconChevron` showed, ported:

```js
if (props.displayDirection != null && props.displayDirection === "down") {
```

## Decision

**A `Some(p)` whose test is `===` of the value and a string, a number or
a `bool` is that test alone:** `d === "Up"` of `Some(Direction::Up)`, and
`o === 0` of `Some(Ordering::Equal)`. Where the test reads inside the
value, `s.TAG === "Line"` of a variant with fields, `s != null` stays, and
where the value is boxed (ADR 0051), as a generic `Option`'s may be.

## Why

- **It's the JS a person writes**, and it's exact: `undefined === "Up"` is
  `false`, as `None` matches no `Some`. A compiler test runs each arm, and
  `None`, of a unit variant's `Option` and of a variant with fields'.
