# 0171. An `i128` or a `u128` is a BigInt, wrapped to 128 bits

Status: Accepted. Extends [0086](0086-64-bit-integers.md).

## Context

ADR 0086 made an `i64` or a `u64` a BigInt and left 128 bits out. The
crates a shared model depends on implement their traits for every integer,
`u128` included: num-traits, which chrono and rust_decimal use, num-conv,
which time uses, zerofrom, which url uses, and serde_core. Each was an
error at its first `u128`, though a program never used one.

## Decision

**An `i128` or a `u128` is what an `i64` or a `u64` is, at 128 bits:** a
BigInt, `5n`, each result wrapped by `BigInt.asIntN(128, ..)` or
`BigInt.asUintN(128, ..)`, every operation and method of ADR 0086 the
same, with its width where it counts:

| Rust | JS |
|---|---|
| `a << n` | `BigInt.asUintN(128, a << (BigInt(n) & 127n))`, masked to 127 |
| `x.pow(e)`, `leading_zeros()`, `count_ones()` | `$bigPow(x, e, 128)`, `$bigLeadingZeros(x, 128)`: the helper's width, 64 by default |
| `checked_shl(n)`, `rotate_left(n)`, `count_zeros()` | of 128 bits |
| `to_le_bytes()`, `from_be_bytes(b)` | 16 bytes, as two 64-bit halves |
| `u128::MAX` | `340282366920938463463374607431768211455n` |

- **The compiler keeps a constant as an `i128`,** which holds every
  `i128` but not a `u128` past `i128::MAX`: a `u128`'s is its bits, and a
  type's range ends at a `u128`, which `u128::MAX` needs.
- **A float's `to_int_unchecked::<T>()` is its `as T`,** which is it
  wherever Rust defines it: in range, as num-traits checks first.
- **Still errors:** a 128-bit integer in JSON, read or written. serde_json
  reads one by its own rules, which aren't checked here yet.

## Why

- **It's Rust's answer:** the `integers_128` corpus case compares the
  literals at both ends, an FNV-1a hash of 128 bits, wrapping, checked,
  saturating and overflowing arithmetic, shifts, rotations, the bit
  counts, casts to and from narrower integers and floats, `try_from`,
  bytes, parsing, a range `match` and sorting with native Rust.
- **It's the JS a person writes:** a BigInt, as ADR 0086's is.
