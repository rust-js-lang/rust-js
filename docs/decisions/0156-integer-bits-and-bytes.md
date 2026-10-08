# 0156. An integer's bits rotated, its bytes, and a float's bits

Status: Accepted. (Amended: `leading_ones`, `trailing_ones`, `swap_bytes`,
`reverse_bits`, `to_be`, `from_be`, `to_le` and `from_le`, the last two
itself on little-endian wasm32, `checked_div_euclid` and
`checked_rem_euclid` of every integer, by its bits as an unsigned BigInt's
digits, `$swapBytes(x, bits, signed)`; and a float's `recip()` and
`to_le_bytes()` and the like, by its bits' bytes. num-traits' `PrimInt`
and `Float` asked them.) Extends [0086](0086-64-bit-integers.md),
[0122](0122-f32.md) and [0155](0155-integer-families.md).

Case: C, B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Hashes, checksums, random number generators and binary formats rotate an
integer's bits, take its bytes and read a float's bits:
`rotate_left`, `rotate_right`, `to_be_bytes`, `to_le_bytes`,
`to_ne_bytes`, the `from_*_bytes`, `to_bits` and `from_bits` were errors.

## Decision

| Rust | JS |
|---|---|
| `x.rotate_left(n)`, `rotate_right(n)` | `$rotateBits(x >>> 0, n, 32, true) \| 0` |
| `x.to_be_bytes()`, `to_le_bytes()` | `$toBytes(x, 4, false)`, `$toBytes(x, 4, true)`: an array of `u8`s |
| `u32::from_be_bytes(b)` | `$fromBytes(b, 4, false, false)` |
| `x.to_bits()`, `f64::from_bits(b)` | `$floatToBits(x, 8)`, `$floatFromBits(b, 8)` |

- **A rotation is of the integer's unsigned bits,** its sign bit among
  them, then read back as its type: `(-2i32).rotate_left(1)` is -3. A
  number's is by powers of two, which are exact, an `i64`'s by BigInt
  shifts.
- **Bytes are a `DataView`'s,** which wraps a negative value to its bits
  and orders them either way. **Native order is little-endian,** as
  rustc's target, wasm32, is (ADR 0090).
- **A NaN's bits are JS's:** engines don't keep a NaN's sign or payload, so
  `to_bits` of one may differ from Rust's, as `is_sign_negative` does (ADR
  0154).

## Why

- **It's exact:** the `integer_bits` corpus case compares each with native
  Rust, in 8, 16, 32 and 64 bits, signed and not, past the width, and a
  hash and a generator written with them.
- **It's the JS a person writes:** a `DataView` is how JS reads bytes.

## Consequences

- `swap_bytes`, `reverse_bits`, `leading_ones` and `trailing_ones` are
  still errors.
