# 0498. `xor` and `zip` drop what they give up

Status: Accepted. Amends [0496](0496-and-drops-what-it-gives-up.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`a.xor(b)` of two `Some`s gives `None`, dropping both values; `a.zip(b)`
of a `Some` and a `None` gives `None`, dropping the one value. Of values
with a destructor, each was refused (ADR 0496).

## Decision

**`xor` and `zip` drop what they give up after what they make: `xor` both
values where both are `Some`, `zip` the one where the other is `None`.**
One helper, `unpaired`, makes either.

```js
const value = a != null ? (b == null ? a : undefined) : b;
if (a != null && b != null) {
  loudDrop_drop(a);
  loudDrop_drop(b);
}
```

## Why

- **It's std's**: `unpaired_option_drops` gives `xor` and `zip` values with
  a destructor of each pairing, with native Rust.
