# 0627. A number literal keeps its base and its exponent

Status: Accepted. Extends [0011](0011-numbers.md). Amended: a constant
expression of literals is as written where JS reads it as its value.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: how a number is
written, not what it is.

## Context

React's feature flags write `export const ownerStackLimit = 1e4;`. In
Rust, `pub const OWNER_STACK_LIMIT: f64 = 1e4;` was `10000`: a number was
always JS's own digits of its value, so a program's `0xdeadbeef`, `0b111`
or `0x40` was `3735928559`, `7`, `64`, what a person reading bits doesn't
write.

## Decision

**A number literal written with a base, `0x`, `0o` or `0b`, or an
exponent, `1e4`, is written as the program wrote it**, without Rust's
type suffix and separators, which aren't JS's (`0x811c_9dc5u32` is
`0x811c9dc5`), where JS reads that text as the same value. Any other
literal is JS's digits as before, `5000` of `5000.0`, whose `.0` Rust
needs and JS doesn't.

Where a number's literal is, its span says: a MIR constant's is its
literal's, and a `const`'s value its initializer's. The printer reads the
program's text there (`as_written`); text that isn't a literal, or a
literal of another value, is JS's digits.

## Why

- **Generated code is a product**: the base a program chose says what the
  number is for, bits or a count, and JS has the same syntax.
- **Only what JS writes alike**: a whole float's `.0` and a suffix are
  Rust's, so those keep JS's digits.

## Alternatives

- **Every literal as written**: `5000.0` and `1_000_000` in the JS, which
  no JS developer writes.
- **Separators kept**: JS has them, but where Rust allows one, `0x_ff`,
  JS doesn't.

## Consequences

- Seventeen corpus programs and two examples write their bits as their
  Rust does, `0xdeadbeef`, `[0x40, 0x49, 0x0f, 0xdb]`, `0b111`.

## What holds it

- `test/lowering-numbers.test.ts`, "a number literal keeps its base and
  its exponent".
- `src/to_oxc/expressions.rs`'s test of `as_written`, of a value its text
  isn't.
- Mutations `number-as-written-unread`, `constant-number-unplaced`,
  `const-number-unplaced`.

## Amendment: a decimal's leading zeros

Rust reads `00e+10` as 0; a JS module refuses it, as it does `01e5`, a
legacy octal's start, so rust-js wrote a module it couldn't read (rustc's
`lexer/floating-point-0e10-issue-40408.rs`). A decimal literal's leading
zeros are dropped, one kept before its point or its exponent: `0e10`,
`7.5e2` of `007.5E2`. The case is "results would change" (ADR 0262):
JavaScript has no spelling of it, so it's rewritten. Held by
`test/lowering-numbers.test.ts`, "a number literal keeps its base and its
exponent", and the mutation `literal-leading-zeros-kept`.

## Amendment: a constant expression of literals

Case ([0262](0262-when-rust-and-js-disagree.md)): A: how a number is
written, where JS reads it as the same value.

react-dom-bindings' EventSystemFlags writes `export const
IS_NON_DELEGATED = 1 << 1;`; its port's `pub const IS_NON_DELEGATED: i32 =
1 << 1;` printed rustc's value, `2`, which says less of what each flag is.

**A constant's value, where its source is an expression of number
literals and operators that JS reads as that value, is written as the
program wrote it:** `1 << 1`, `(1 << 4) - 1`, `-(1 << 3)`, `(0b1 << 3) |
0x1`. The printer (`as_written_expression`, `src/to_oxc/expressions.rs`)
reads the value's source, takes integer literals (each as a literal is
written, `as_written`), unary `-` and `~`, `* + - << >> & ^ |` and
parentheses, evaluates it as JS does, by JS's precedence, `<<`, `>>` and
the bitwise operators of 32-bit integers, and writes it only where that's
rustc's value exactly; the layout drops the parentheses it needn't keep.
So one JS reads otherwise is the value as before: `1 << 31` of a `u32` is
`2147483648`, JS's `1 << 31` being negative; and `!0`, whose JS is `~0`,
is `-1`, as is anything else, a cast, a name, `/` or a literal past
2^53.

Alternatives: the expression lowered as Rust's operators, `(1 << 31) >>>
0`, writes what the program didn't; a rule by the expression's type
rather than by its value, which a mistake would make wrong, where this
can only fall back to the value. `test/lowering-numbers.test.ts` "a
constant expression of literals is as written"; mutations
`written-expression-unchecked`, `written-expression-shift-unmasked` and
`written-expression-precedence-flat`.
