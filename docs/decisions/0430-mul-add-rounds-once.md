# 0430. `mul_add` rounds once

Status: Accepted. Extends [0122](0122-f32.md) and
[0365](0365-js-math-where-rust-differs.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a fused
multiply-add's result is the program's, and JS has none, so it's Rust's,
paid for in a helper.

## Context

`a.mul_add(b, c)` is `a * b + c` rounded once, as the hardware's fused
multiply-add gives it, where `a * b + c` rounds `a * b` first. The two
differ where that first rounding loses what `c` cancels, `0.1.mul_add(10.0,
-1.0)` is `5.551115123125783e-17` where `0.1 * 10 - 1` is `0`; where `a *
b` is past a float's range and the sum isn't; and at a tie. Numerical code,
geometry and graphics call it for the precision. JS has no `Math.fma`, and
rust-js refused the call, and with it rustc's `intrinsics-math.rs`.

## Decision

**`a.mul_add(b, c)` is `$mulAdd(a, b, c)`, of `f32`s `$mulAddF32(a, b, c)`,
which round `a * b + c` once, to nearest, a tie to even.**

```js
const error = $mulAdd(0.1, 10, -1); // 5.551115123125783e-17
```

- **Exact, in a `BigInt`:** each operand is an integer times a power of
  two, so the sum is one, exactly; `$fused` rounds it to a float of the
  type's significant bits and least step, an `f64`'s 53 and `2 ** -1074`,
  an `f32`'s 24 and `2 ** -149`: a subnormal's tie too.
- **An `f32`'s is rounded once to an `f32`,** not to an `f64` first: the
  `f64` of a tie an `f32` can't hold plus less than an `f64` holds is the
  tie, which rounds to even, where Rust's rounds past it.
- **Of an infinity or a NaN, and of a sum that's exactly zero,** JS's `a *
  b + c` is the answer, its zero's sign IEEE's.

## Why

- **It's Rust's answer:** the `mul_add` corpus case gives what rounding
  `a * b` first loses, a product past `f64::MAX` whose sum isn't, both
  subnormal ties, each zero's sign, infinities and NaN, and an `f32` tie
  tipped by less than an `f64` holds, bit for bit with native Rust.
- rustc's `intrinsics/intrinsics-math.rs` passes.

## Alternatives

- **`a * b + c`.** Wrong where the precision `mul_add` is called for
  matters, which is why it's called.
- **Dekker's error-free product and a two-sum.** Fast, but its correct
  rounding at a tie, a subnormal and an overflow is a proof per case;
  exact integers are right by construction, at a `BigInt`'s cost.

## Consequences

- A `mul_add` allocates a few `BigInt`s; a hot loop of them is slower
  than the hardware's, as `$powi` is than `**`.

## Amendment: a finite product given an infinity is that infinity (#417)

Of an infinity or a `NaN` among its operands, `$fused` gave JS's own
`a * b + c`, taken to be what rounding once gives. It isn't where `a` and
`b` are finite and `c` is infinite: the product is exact, so finite, and
the sum is `c`, but JS's `a * b` may be past the range,
`f64::MAX.mul_add(2.0, f64::NEG_INFINITY)` its `Infinity - Infinity`,
`NaN` where Rust gives `-inf`. A wrong answer, case C
([0262](0262-when-rust-and-js-disagree.md)): such a `c` is the answer;
any other non-finite operand is JS's `a * b + c` as before, an infinite
product's sum and a `NaN`'s.

Held by corpus `mul_add` against native Rust, of each sign and of an
`f32`; mutations `fused-infinite-addend-unfused` and
`fused-infinite-addend-whatever-product`.
