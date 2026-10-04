# 0177. A `NonZero` integer is its number

Status: Accepted. Extends [0011](0011-numbers.md), [0086](0086-64-bit-integers.md)
and [0030](0030-option.md).

## Context

A type that can't be zero lets `Option` of it take no more room, so crates
keep counts, ids and packed fields in one: chrono's dates are a
`NonZeroI32`, deranged's bounds a `NonZeroU8`.

```rust
let year = NonZeroI32::new(2024).unwrap();
```

std keeps a `NonZero<u8>` as a pattern type inside a struct of its own,
`(u8) is 1..`, which rust-js had no value for. That stopped chrono, at 333
places, and deranged, so time.

## Decision

**A `NonZero<T>`, and the pattern type std keeps it in, is its number, as a
`T` is:** a number, or a BigInt of 64 bits and more.

| Rust | JS |
|---|---|
| `NonZero::new(n)` | `n === 0 ? undefined : n`, its `None` of `0` (ADR 0030) |
| `NonZero::new(7)` | `7` |
| `n.get()`, `NonZero::new_unchecked(n)` | `n` |
| `NonZeroU8::MIN` | `1`, the number inside std's own structs |
| `match n { SEVEN => .. }`, a constant of one | `n === 7`, its number's |
| `{}`, `{:?}`, `==`, `<`, `n \| 8`, its methods | its number's |
| `"0".parse::<NonZeroU8>()` | `$nonZeroOk(..)`: `Err(ParseIntError { kind: Zero })` |
| `NonZeroU8::try_from(0u8)` | `Err(TryFromIntError(Zero))` |

- **Its number in every way but zero:** each of its methods is the
  number's, which gives the same answer on a number that isn't zero; what
  makes one checks for zero, as std's do.
- **`Num::of`, which has no `TyCtxt`, asks rustc's own for the thread**
  whether a type is `NonZero`, as only recognition names it.

## Why

- **It's exact:** the `nonzero` corpus case makes one of a constant and of
  a value, `None` of `0`, a 64-bit one, `MIN`, `|`, `<`, `==`, its methods,
  matching constants, sorting, parsing and converting with their errors of
  `0`, with native Rust.
- **It's the JS a person writes:** a number, as the `Option` around it is
  the number or `undefined`.
