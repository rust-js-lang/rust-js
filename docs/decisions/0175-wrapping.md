# 0175. `std::num::Wrapping` is a number in a `[x]`, its arithmetic wrapped

Status: Accepted. Extends [0011](0011-numbers.md), [0086](0086-64-bit-integers.md)
and [0108](0108-generic-operators-and-into.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`Wrapping<T>` is a number whose arithmetic wraps, in debug builds too.
num-traits implements its traits for every `Wrapping`, and its `pow` takes
one where any `T: Mul` goes:

```rust
pow_impl!(Wrapping<u8>);
```

`a * b` of a `Wrapping` was an error, as was its dictionary, which stopped
num-traits, so chrono and rust_decimal.

## Decision

**A `Wrapping` is what any tuple struct is, `[x]`, and its operators its
number's, wrapped as release Rust wraps them (ADR 0011):**

| Rust | JS |
|---|---|
| `Wrapping(200u8) * Wrapping(2)` | `[(a[0] * 2) & 255]` |
| `c += Wrapping(1)` | `c = [(c[0] + 1) \| 0]`: a new `[x]` for its place |
| `Wrapping(1u8) << 9`, a `usize` amount | `[(a[0] << (9 & 7)) & 255]`: masked, as `wrapping_shl` masks it |
| `-w`, `!w` | `[-w[0] ...]` |
| `{}` and `{:?}` | its number's, as std's impls show it |
| `==`, `<`, `max` | its number's, as its derives compare it |
| a `T: Mul` given a `Wrapping<u8>` | `{ mul: (a, b) => [(a[0] * b[0]) & 255] }` |

- **`a += b` writes a new `[x]`,** not into the old one, which a copy may
  share where its type isn't otherwise changed in place (ADR 0052).
- **`[x][0]` is written `x`,** as any one-item array literal indexed by `0`.

## Why

- **It's exact:** the `wrapping_type` corpus case compares each operator,
  assignment, shift, `{}`, `{:?}`, `==`, `<` and `max`, and `pow`, a sum and
  `==` through generic code's bounds, with native Rust.
- **It's the JS a person writes:** the number's arithmetic, wrapped, in the
  one-item array every tuple struct is.
