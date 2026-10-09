# 0302. An `f64` known to be a whole number in range is cast as it is

Status: Accepted. Amends [0086](0086-64-bit-integers.md): when a cast
from a float saturates.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`as` from a float to an integer saturates, `NaN` to 0 (ADR 0086):
`$f64ToInt(x, 0, 4294967295)`. react.dev's CodeBlock finds a column with
`indexOf`, which gives an `f64` since it may be -1, panics where it's -1,
and keeps it as a `u32`:

```rust
let index = string::index_of(line, &substr);
if index == -1.0 {
    panic!("Could not find: '{substr}'");
}
InlineHighlight { start_column: index as u32, .. }
```

The original writes `startColumn: index`: at the cast, `index` is a whole
number, 0 or more, and below the line's length, which rust-js takes as a
`u32` already (`string::length`).

## Decision

**A cast to an integer of 32 bits or fewer, of an `f64` variable known
there to be a whole number in the integer's range, is the variable.**

- What a variable may hold is what its `let` and each `=` give it: a whole
  number, an integer `as f64`, or what a binding marked
  `#[rust_js::position]` gives, a position in what it's called on or -1,
  as builtins' `string::index_of`, `index_of_from` and `last_index_of`.
  Anything else it's given, or a `+=` or a `&mut` of it, and it's not
  known.
- A test of it against a constant narrows it where the test holds: in an
  `if`'s branches, and after `if index == -1.0 { panic!(..) }` in its block,
  so -1 or more is 0 or more. Not where it's set again after the test.
- A cast to a 64-bit integer is a BigInt still.

## Why

- **It's the JavaScript a person writes**: `startColumn: index`.
- **It's the same program**: a whole number in range is what saturating
  gives of it.
