# 0154. Number methods, and parsing that stops where Rust's stops

Status: Accepted. Extends [0063](0063-text.md), [0064](0064-numbers.md),
[0086](0086-64-bit-integers.md) and [0122](0122-f32.md).

## Context

App code stopped at number methods: `clamp`, `from_str_radix`,
`div_ceil`, a float's `signum`, `fract`, `to_radians`, `to_degrees` and
`is_sign_negative`, and an integer's `ilog2`, `ilog10`, `isqrt`, `midpoint`
and `count_zeros`.

Reading std's parser for `from_str_radix` found `parse` wrong. It checked
every digit, then the range, where std reads one digit at a time and stops
at the first problem:

```rust
"999x".parse::<u8>() // Err(PosOverflow): 999 is too large before the `x`
"26x".parse::<u8>()  // Err(InvalidDigit): the `x` is read before 260 is checked
```

rust-js answered `InvalidDigit` for both.

## Decision

**Each is what std's does, by std's own steps:**

| Rust | JS |
|---|---|
| `x.clamp(lo, hi)` | `$clamp(x, lo, hi)`, a float's `$clampFloat(x, lo, hi, $debugF64)` |
| `T::from_str_radix(s, r)` | `$parseInt(s, min, max, r)`, an `i64`'s `$parseBig` |
| `a.div_ceil(b)` | `$divCeil(a, b)` |
| `x.signum()` of a float | `$signum(x)`: 1 or -1 by its sign, -0's -1 |
| `x.fract()` | `x - Math.trunc(x)` |
| `x.to_radians()`, `to_degrees()` | `x * (Math.PI / 180)`, `x * (180 / Math.PI)`; an `f32`'s by std's `f32` constants |
| `x.is_sign_negative()` | `$signNegative(x)` |
| `x.ilog2()`, `ilog10()` | `$ilog(x, 2)`, `$ilog(x, 10)`: by dividing, exact |
| `x.isqrt()` | `$isqrt(x)` |
| `a.midpoint(b)` of integers | `Math.trunc((a + b) / 2)`, an `i64`'s `(a + b) / 2n` |
| `x.count_zeros()` | `32 - $countOnes(x)` |

- **Parsing reads a digit, then checks what's so far times the radix, then
  adds the digit,** as std's loop does, so the error is std's.
- **Panics are std's:** an integer's bounds the wrong way round,
  `min > max. min = 5, max = 1`; a float's, or a NaN bound, `min > max, or
  either was NaN. min = NaN, max = 1.0`; `argument of integer logarithm must
  be positive`; `argument of integer square root cannot be negative`; a
  radix past 36, `from_ascii_radix: radix must lie in the range [2, 36] -
  found 37`; `div_ceil` by zero, `attempt to divide by zero`.
- **A NaN's sign isn't kept:** JS engines don't keep it, Bun never and Node
  sometimes, so `is_sign_negative` of a NaN is `false`, as `f64::NAN`'s is,
  and of `-f64::NAN` too, where Rust's is `true`. It's in the semantics
  page's list of differences.
- **Still errors:** `mul_add`, which JS can't fuse, and a float's
  `midpoint`, whose own algorithm isn't followed yet.

## Why

- **It's exact:** the `number_methods` corpus case compares each with
  native Rust, `-0.0`, NaN, `f32`s and 64-bit integers included;
  `parse_error_order` compares where parsing stops; four `run-fail` cases
  compare the panics.
- **It's the JS a person writes:** `$clamp(level, 1, 5)`, `x * (Math.PI /
  180)`.

## Consequences

- `parse` errors of strings both too long and wrong, `"999x"`, are Rust's.
