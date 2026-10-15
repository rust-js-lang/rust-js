# 0393. An enum's discriminant, `mem::discriminant` and a cast, one way

Status: Accepted. Extends [0013](0013-fieldless-enums.md), [0284](0284-discriminated-unions.md)
and [0331](0331-dyn-any.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): which variant a value
is, as rustc numbers it, is what the program compares and prints.

## Context

`mem::discriminant(&e)` tells which variant `e` is, whatever its fields:
code compares two values' kinds with it, and keys a map by it. rust-js
refused it. A cast of a fieldless enum, `color as i32`, was the one place
that worked a discriminant out, `["Red", "Green"].indexOf(color)`, and only
of a string enum: a tagged enum's fieldless variant is an object, `{ kind:
"On" }` (ADR 0284), whose `indexOf` is -1. rustc folded the constant casts
the corpus had, so a cast where it runs was never tried.

## Decision

**An enum value's discriminant is `discriminant_value`'s, one place: by its
variant's name where its value says it once (`named_discriminant`), a
string enum's value or a tagged one's tag; else each variant's test in
turn. A `Discriminant<T>` is that number, as a `TypeId` is its key.**

```js
// A string enum's, or a tagged one's, by its name:
["On", "Off"].indexOf(light.kind);
({ Low: 3, High: 10 })[code];
// One with fields, by each variant's test:
shape.TAG === "Circle" ? 0 : shape.TAG === "Square" ? 1 : 2;
```

- **A cast is `named_discriminant`'s**, a BigInt where it's read as one;
  of an enum whose value doesn't name its variant, which Rust casts none
  of, an error.
- **The numbers are rustc's**, signed as the enum's type is
  (`discriminants`).
- **A `Discriminant<T>` is its number**: `==` is `===`, a map keys by it,
  `{:?}` is `Discriminant(1)`, as std shows it, and a constant of one,
  `const EMPTY: Discriminant<Shape>`, is its number, read out of the value
  tree inside std's struct, as a `NonZero`'s is (`innermost`, the one
  unwrapping of all three such).

## Why

- **It's which variant**, exact for any enum rust-js has.
- **It's tested**: the corpus's `enum_discriminant.rs` compares kinds,
  counts a map by them, shows them and compares with a constant;
  `tagged_enum_values.rs` casts a tagged enum where it runs; rustc's
  `enum-discriminant/discriminant_value-wrapper.rs` and
  `consts/const_discriminant.rs` pass.
