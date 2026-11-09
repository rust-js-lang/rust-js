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

## Amendment: of a type only a caller knows, refused (#1014)

`mem::discriminant` of a type parameter's value, or of an associated
type's only a caller knows, `fn same_kind<T>(a: &T, b: &T) -> bool {
discriminant(a) == discriminant(b) }`, was the constant `0` of a type
that isn't an enum, so `same_kind(&Circle(1.0), &Square { .. })` was
`true` where Rust's is `false`. Generic code is made once, and its `T`'s
JS varies by type, a fieldless variant's name, `{ TAG }`, a nullable
`Option`'s value or `undefined`, a struct's array: no test of the value
alone tells which variant it is. It's refused where it's asked, "`mem::
discriminant` of a type only a caller knows"
([0262](0262-when-rust-and-js-disagree.md)'s fourth case, the user's
decision, 2026-11-09). Its support, a discriminant function each caller
gives for the type it has, as callers give drop functions, is #1040. Held
by the diagnostics test `a discriminant in generic code`, the guide's
limitations and the mutation `discriminant-of-a-callers-type-zero`; the
net's `generic enum_discriminant` row, now refused, which the net counts as no
wrong answer, is taken off.

## Amendment: a discriminant is its type's value, a wide one a BigInt (#1131)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results changed.
A discriminant of a `repr` 64 bits wide or wider is a BigInt in JS, as
its type's every value is, but three places made it otherwise:

- Read by MIR as a value, the discriminant a library's derived `Hash`
  reads where its consumer compiles it, it was a number, `-5` of a
  `#[repr(i64)]` enum's `Low = -5`, which `write_i64` threw on: `TypeError:
  Cannot convert -5 to a BigInt` where native Rust hashes it (found by the
  metamorphic net's `library` rewrite of `hash_values`, once the net ran
  the corpus's `native: wasm32` cases).
- A `u128`'s past `i128::MAX` was kept as an `i128`, wrapped negative: of
  `enum Endless { First = 1, Last = u128::MAX }`, `First < Last` was
  `false`, `Last as u128` of a variable `-1`, and its `Discriminant`
  `Discriminant(-1)`; and a `mem::discriminant` of one was a number.
- A derived `PartialOrd` of an enum with fields ordered its variants by
  their discriminants' bits, a negative one's last: `High(1) < Low` where
  `Low = -5` comes first.

A discriminant's value has one answer now, `Discr` (`lower.rs`'s
`discriminants`, and `Discr::of` of each of rustc's): a signed `repr`'s
sign-extended, an unsigned one's as it is, ordered by its value, each made
JS by `Discr::expr`, a BigInt where its type's is. A library's enum isn't
in its manifest; its consumer reads its discriminants from rustc's
metadata, as its own. Held by corpus case `discriminants_past_32_bits`
(`i64`, `u64`, `u128`), the crate pair `wide_discriminants` (`i64`, `u64`,
`i128`, `u128`, hashed, compared, shown, cast and told apart through a
library), the net's `library` rewrite of `hash_values`, off its known
list, and mutations `discriminant-number-not-big`, `discr-unsigned-wrapped`,
`discriminant-of-a-wide-repr-a-number` and `variants-ordered-by-bits`;
`discriminant-big-as-number` retargeted at `Discr::expr`'s caller.
Comparing a library's enum, and a stable sort in generic code of one, stay
refused, as before.
