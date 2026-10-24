# 0722. A `#[repr(..)]` enum's variant is its number

Status: Accepted. Extends [0013](0013-fieldless-enums.md) and
[0393](0393-an-enums-discriminant.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: which JS value a
variant is, nothing a program observes but through its own `==`, order,
`as`, `{:?}` and serde, each kept Rust's.

## Context

React's `shared/ReactPerformanceTrackProperties` keeps an array's kind
as a number, `const EMPTY_ARRAY = 0; const COMPLEX_ARRAY = 1; ..`. Its
port's `enum ArrayKind { Empty, Complex, Primitive, Entries }` printed
`'Empty'`, `'Complex'`, as every fieldless variant is its name (ADR
0013), and so did one written `#[repr(u8)] enum ArrayKind { Empty = 0,
.. }`: the Rust that says each variant's number had no way to make it
one in JS. A port had only `const EMPTY_ARRAY: u8 = 0;`, no enum.

## Decision

**A fieldless enum under a `#[repr(..)]` of an integer of 32 bits or
fewer is its numbers**, as rustc numbers its variants, written or not:
`ArrayKind::Empty` is `0`. `bindings::variant_tag` is the one answer to
what a variant is in JS, and `bindings::numbered` whether an enum's are
numbers; each question of its value asks them:

- **a test, a `match` and a `switch`** compare the number, `kind === 0`;
- **`as u8`, a `Hash` and `mem::discriminant`** are the value itself
  (`named_discriminant`);
- **`==`, a `HashMap`'s key and `contains`** are JS's `===` (`js_eq`);
- **a derived order** is JS's `<`, by the numbers, as the derive orders
  by discriminants (`is_primitive_ord`); a hand-written one is its own;
- **`{:?}`** is its name, `["Empty", "Complex"][kind]` where they count
  from 0, else `{ "-1": "Low", 7: "Mid" }[level]`;
- **its `.d.ts`** is `0 | 1 | 2 | 3`; **an untagged enum** tells it as a
  number; **serde** writes and reads its name, as serde does, and makes
  the number of it (`de.rs`, by `variant_tag`);
- **a function giving each variant its name** is no longer its value
  (`gives_own_name`, ADR 0264), which it is only of a string enum.

A plain fieldless enum is its names, as before. So is one of a 64-bit
or 128-bit `repr`, whose integers are BigInts (ADR 0086), one with a
variant named by `#[rust_js::name]`, a tagged or untagged one, and std's.

## Why

- **The `repr` says the number**: what the Rust writes is what the JS
  holds, as React's constants are, and what `as` gives is the value.
- **One place**: a variant's value was `variant_name`'s string in each
  consumer; each now asks `variant_tag`.

## Alternatives

- **Every fieldless enum its number**: what reads `"Ascending"` in a log
  reads `0`; ADR 0013's reasons hold where the program didn't ask for an
  integer.
- **Only with every discriminant written**: `#[repr(u8)]` already fixes
  the numbers rustc gives the rest.

## Consequences

- `enum_order`'s and `hash_values`'s snapshots read the numbers:
  `writeU8(state, small)`, and a constant comparison folded.

## What holds it

- `test/lowering-structs-enums.test.ts`, "a repr enum's variant is its
  number"; `test/corpus/repr_enum_numbers.rs` against native Rust; the
  serde case "a repr enum serializes and deserializes by its variants'
  names".
- Mutations: `numbered-enum-named`, `numbered-enum-wide`,
  `numbered-discriminant-tested`, `numbered-debug-table`,
  `numbered-gives-own-name`, `numbered-ordered-by-name`,
  `numbered-declared-names`, `numbered-eq-structural`,
  `numbered-payload-untold`, `numbered-read-as-name`.
