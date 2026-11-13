# 0177. A `NonZero` integer is its number

Status: Accepted. Extends [0011](0011-numbers.md), [0086](0086-64-bit-integers.md)
and [0030](0030-option.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

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

## Amendment: read from JSON as serde reads it (2026-11-13, #1538)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, the JS threw where
Rust runs.

Reading a `NonZero<u32>` from JSON named its reader by its type,
`$json["NonZero<u32>"]`, which `$json` hasn't: `TypeError: read is not a
function` where the program ran (P1). It's now `$json.nonZero($json.u32)`:
its number's reader, told it's serde's `NonZero` of it, `read(json, true)`,
so the decoder's `int` says what's expected as serde does, `a nonzero u32`,
of a number out of its range, a float or a string, and refuses zero,
`invalid value: integer `0`, expected a nonzero u32`, each where
serde_json says. A number is named by its type only of a primitive; any
other type rust-js holds as a number is refused, as a `Duration` is.
`test/architecture-json.test.ts` holds each reader the lowering names to
one `$json` has.

The serde-values test `a NonZero is read as serde reads it` holds each
width, zero, out of range, a float, a string, from text and from a
`Value`, in a field, an `Option`, a `Vec` and a map's keys, against native
serde_json. Mutations `non-zero-reader-unmade`, `non-zero-zero-read` and
`non-zero-read-as-its-number`, caught.

