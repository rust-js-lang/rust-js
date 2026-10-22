# 0627. A number literal keeps its base and its exponent

Status: Accepted. Extends [0011](0011-numbers.md).

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
