# 0155. An integer's checked, wrapping, overflowing and saturating families

Status: Accepted. Extends [0064](0064-numbers.md), [0086](0086-64-bit-integers.md)
and [0154](0154-number-methods.md).

## Context

`checked_add`, `wrapping_mul` and `saturating_sub` compiled, but each
family had gaps: `checked_neg`, `checked_rem`, `checked_abs`,
`checked_shl`, `wrapping_neg`, `wrapping_div`, `wrapping_shl`,
`wrapping_pow`, every `overflowing_*`, `saturating_pow`, and
`is_positive`/`is_negative` were errors. Hashes, counters and ring buffers
use them.

## Decision

**Each family is one answer seen four ways:** the exact result, in range or
not. `checked_*` is it or `None`, `wrapping_*` it wrapped, `overflowing_*`
both, `saturating_*` it clamped.

| Rust | JS |
|---|---|
| `x.checked_neg()`, `checked_abs()` | `$checked(-x, lo, hi)`, `$checked(Math.abs(x), lo, hi)` |
| `a.checked_rem(b)` | `$checkedRem(a, b, MIN)`: `None` of 0, and of `MIN % -1` |
| `a.checked_shl(n)` | `n < 32 ? a << n : undefined` |
| `x.wrapping_neg()`, `wrapping_abs()`, `wrapping_pow(e)` | `-x \| 0`, as `abs` and `pow` wrap already |
| `a.wrapping_div(b)`, `wrapping_rem(b)` | `$wrappingDiv(a, b, MIN)`, `$wrappingRem(a, b, MIN)` |
| `a.wrapping_shl(n)` | `a << n`, the amount masked, as `<<` is |
| `a.overflowing_mul(b)` | `$overflowing(Math.imul(a, b), a * b, lo, hi)`: `[wrapped, overflowed]` |
| `x.overflowing_neg()` | `[-x \| 0, x === MIN]`, an unsigned's `x !== 0` |
| `x.saturating_pow(e)` | `$saturatingPow(x, e, lo, hi)` |
| `x.is_positive()`, `is_negative()` | `x > 0`, `x < 0` |

- **The exact result decides:** in range of the type, `lo` to `hi`; a
  product past 2^53, rounded, is far out of range either way. An `i64`'s
  is a BigInt, exact.
- **`MIN / -1` and `MIN % -1` overflow:** `checked_*` is `None`,
  `wrapping_div` `MIN`, `wrapping_rem` 0. A zero divisor still panics, as
  `/` and `%` do, `attempt to calculate the remainder with a divisor of
  zero`.
- **An integer is never `-0`:** `0.wrapping_div(-5)` is 0.
- **A negative base to an odd power saturates at `MIN`.**

## Why

- **It's exact:** the `integer_families` corpus case compares each with
  native Rust at `MIN`, `MAX`, 0 and past the width, in 8, 32 and 64 bits;
  `wrapping_rem_zero` the panic. The `wrapping_unary` gap case passes now.
- **It's the JS a person writes:** `$overflowing(Math.imul(a, b), a * b,
  ..)`, the wrapped value and the exact one side by side.

## Consequences

- `swap_bytes`, `reverse_bits`, `leading_ones`, `trailing_ones` and the
  `overflowing_` shifts are still errors.
