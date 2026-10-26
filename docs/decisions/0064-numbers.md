# 0064. Numbers' methods are `Math`'s, where JS agrees; operators call their impl

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md), [0031](0031-consts.md) and [0052](0052-std-trait-impls.md).
Amended: an unsigned number's `next_power_of_two()` and
`checked_next_power_of_two()` are `$nextPowerOfTwo(n, bits)`: past its
width, `0`, as release Rust wraps, and `None`
(`test/corpus/next_power_of_two.rs`).
Extended by [0887](0887-std-items-that-are-math-are-math.md): a method whose call is `Math`'s function of its
arguments, for every value of its type, is that function taken as a `fn`.

Case: C, D, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A program that computes calls numbers' methods (`x.sqrt()`, `n.pow(2)`,
`a.checked_add(b)`), puts operators on its own types (`impl Add for Vec2`),
names constants on them (`Vec2::ZERO`), and makes grids (`vec![vec![0;
n]; n]`). All of these were errors.

A number is a JS number (ADR 0025): integers are kept in range by wrapping
each result. JS's `Math` has most of Rust's methods, but not all agree:
`Math.round(-2.5)` is `-2`, and Rust's `(-2.5f64).round()` is `-3`.
`7 ** 30` is rounded past 2^53, and `7u32.pow(30)` wraps exactly. And
JS has `-0`, which no integer is: `-4 % 2` is `-0`.

## Decision

**A method that JS has, and gives the same answer, is JS's; one where JS
differs is a helper with Rust's answer:**

| Rust | JS |
|---|---|
| `x.floor()`, `sqrt()`, `abs()`, `trunc()`, `hypot(y)`, `atan2(x)`, … | `Math.floor(x)`, `Math.sqrt(x)`, … |
| `x.ln()`, `x.is_nan()`, `x.is_finite()` | `Math.log(x)`, `Number.isNaN(x)`, `Number.isFinite(x)` |
| `x.powf(y)` | `x ** y` |
| `x.round()` | `$round(x)`: a half away from zero |
| `x.powi(n)` | `$powi(x, n)`: the multiplications Rust's `__powidf2` does, in its order |
| `n.pow(e)` | `$pow(n, e)`: multiplied with `Math.imul`, then wrapped |
| `n.abs()`, `n.signum()`, `a.abs_diff(b)` | `Math.abs(n) \| 0`, `Math.sign(n)`, `Math.abs(a - b)` |
| `a.checked_add(b)`, `checked_div` | `$checked(a + b, lo, hi)`, `$checkedDiv(a, b, min)`: `undefined` is `None` |
| `a.saturating_sub(b)` of a `u32` | `Math.max(a - b, 0)` |
| `a.wrapping_mul(b)` | `Math.imul(a, b)`, as `*` is |
| `a.rem_euclid(b)`, `a.div_euclid(b)` | `$remEuclid(a, b, min)`, `$divEuclid(a, b, min)`; `%` and `/` for unsigned types |
| `n.leading_zeros()`, `trailing_zeros()`, `count_ones()` | `Math.clz32(n)`, `$trailingZeros(n, 32)`, `$countOnes(n)` |
| `v.binary_search(&x)` | `$binarySearch(v, x)`: Rust's search, step for step |
| `vec![0; n]` | `new Array(n).fill(0)` |
| `vec![vec![0; m]; n]` | `Array.from({ length: n }, () => new Array(m).fill(0))` |

- **Exact results are exact.** Integers are below 2^32, so a sum or
  difference is exact in JS, and a product is exact below 2^53. Past that,
  a product is rounded, but it's far out of any range, so `$checked` still
  knows it's out.
- **An integer is never `-0`.** `%`, `/` and `*` can make it in JS, and
  `1.0 / x as f64` would tell. Helpers add `0`, which makes `-0` `0`.
- **`binary_search` finds the index Rust does** among equal items, since it
  halves the same way. It takes integers, `char`s, `bool`s and strings,
  which `<` orders as `Ord` does.
- **`vec![x; n]` fills with `x` when copies of it can't be told apart**
  (ADR 0052). Otherwise each item is its own, as Rust clones it: made again
  when `x` is built from pure parts, as `vec![0; m]` is, or cloned,
  `Array.from({ length: 3 }, () => ({ ...cell }))`.
- **An operator on the crate's own type calls its impl:** `a + b` is
  `vec2Add_add(a, b)`, `-a` is `vec2Neg_neg(a)`, and `c += b` is
  `vec2AddAssign_add_assign(c, b)`. Like `From`, operators had no
  dictionaries, so `T: Add` in generic code was an error, until ADR 0108.
  The bitwise ones are the same, since a set of flags has them: `a | b`
  is `flagsBitOr_bitor(a, b)`, and `f <<= 1u8`
  `flagsShlAssignU8_shl_assign(f, 1)`.
- **A type's own `const`,** `Vec2::ZERO`, is its value where it's used, as
  Rust's is: `{ x: 0, y: 0 }`, and `Vec2::ZERO.y` is `0`. A trait's
  associated constants were still errors, until ADR 0106.
- **Also here:**
  - a derived `Debug` of a fieldless enum is the value, its variant's name
    (ADR 0013), with no function;
  - `for (j, &v) in row.iter().enumerate()` takes `&v` apart like `v`:
    `for (const [j, v] of row.entries())`;
  - numbers are written as JS's `String(n)` writes them: `0.25`, not `.25`,
    and `2.220446049250313e-16`;
  - `{}` of an integer constant is its text: `" 2147483647 "`.
- **Still errors:** `clamp`, `f64`'s `signum`, `mul_add` and `to_degrees`,
  `overflowing_*`, `binary_search_by`, and 64-bit integers.

## Why

- **It's Rust's answer.** Differential tests check each method on the edge
  cases: `i32::MIN`, `u32::MAX`, halves, `-0.0`, NaN, infinities, `1e21`,
  and `i8`s and `u8`s that wrap.
- **It's the JS a person writes** where JS agrees, and a named helper where
  it doesn't, so the difference is visible.

## Alternatives

- **`Math.round`, `Math.pow` and `/` alone:** shorter, but wrong for
  halves below zero, for powers past 2^53, and wherever a `-0` can show.
- **Operators as methods of the type's object** (`Vec2.add(a, b)`). It would
  read well, but a trait impl's functions are named for the trait and the
  type already (ADR 0052), and `Vec2.add` could collide with an inherent
  `add`.
- **A type's constants in its object** (`Vec2.ZERO`). It reads well, but a
  use that changes its copy would need `{ ...Vec2.ZERO }`, which is the
  value written out anyway.

Custom or generic clones, including those in owned fields, use `$repeat`
(ADR 0069): clone `n - 1` times, then move the original. Rebuilding pure parts
is allowed only when cloning is structural.

## Consequences

- JS's `Math.sin`, `Math.exp`, `Math.log` and `**` aren't required to be
  correctly rounded, and neither is the C library Rust calls. They agree on
  the values tested, but may differ in the last bit for others.

## Amendment: the rest of a float's methods

num-traits' `Float` calls a float's `log(base)`, `sin_cos`, `asinh`,
`acosh`, `atanh`, `rem_euclid`, `div_euclid`, `abs_sub` and `copysign`;
each was refused.

- **`x.log(b)` is `Math.log(x) / Math.log(b)`**, as std's is `ln / ln`;
  `sin_cos` is `[Math.sin(x), Math.cos(x)]`, `x` read once; `asinh`,
  `acosh` and `atanh` are `Math`'s.
- **`rem_euclid` and `div_euclid` are `$floatEuclid(a, b, rem)`**, std's
  own steps: the remainder made non-negative, the quotient moved one past
  the truncated one where it was negative. `abs_sub` is `$absSub`, C's
  `fdim`, `0` where `a <= b`.
- **`x.copysign(y)` reads `y`'s sign bit**, `$signNegative`, so a `-0.0`
  gives a negative.
- **A float's `midpoint`** is std's: an `f64`'s, `$floatMidpoint`, halves
  the sum where it can't overflow, else each first; std keeps a tiny one
  whole beside a huge one, whose half absorbs it either way. An `f32`'s
  is `f64`s' sum halved, rounded.

Case: N for each but the transcendental functions, which are the case
under Consequences: neither JS's `Math` nor the C library Rust calls is
correctly rounded, so either may differ in the last bit. The corpus case
`float_methods_more` prints them rounded, and the rest exactly, against
native Rust.

## Amendment: division's overflowing and saturating forms, `is_multiple_of`, and casts of sign

`overflowing_div`, `overflowing_rem`, `saturating_div`, `is_multiple_of`,
`cast_unsigned` and `cast_signed` were refused (#119). Each is written the
same of a number and a BigInt, `either_width_call`: `$overflowingDiv(a, b,
min)`, `wrapping_div`'s and whether `MIN / -1` wrapped; `$saturatingDiv`,
`MAX` of it; `$isMultipleOf`, of a zero divisor whether it's zero, as
std's; and a cast `as` the other type of its width. A zero divisor panics
as `/` does. Held by `test/corpus/int_div_sign_methods.rs`.

`unbounded_shl` and `unbounded_shr`, `next_multiple_of` and
`checked_next_multiple_of`, and `wrapping_rem_euclid` and
`wrapping_div_euclid` were refused too (#119). A shift past the width is
0, or a negative's `-1` shifted right, and below it as `<<` and `>>` are;
`$nextMultipleOf` wraps past the range as overflow does, its checked form
none; the wrapping Euclidean forms are 0 and `MIN` of `MIN` by `-1`, where
the plain ones panic. Held by `test/corpus/int_shift_multiple_methods.rs`.

And the rest of #119: `overflowing_shl`, `overflowing_shr`,
`overflowing_abs` and `overflowing_pow`, the wrapping one and whether it
wrapped; `ilog(base)` and `checked_ilog`, `$ilog` given its base, a base
below 2 panicking as std's does, or none; `checked_isqrt` of a signed
integer, none of a negative; the `_add_signed` and `_add_unsigned` forms,
checked, wrapping, saturating and overflowing, of their exact sum; and
`checked_signed_diff`; and a byte's `eq_ignore_ascii_case`. Held by
`test/corpus/int_overflow_sign_methods.rs`.

## Amendment: a variable's square is `x * x` (#336)

trekhleb/javascript-algorithms' `squareRoot`, ported, read
`Math.abs(number - $powi(root, 2))` where upstream writes `root ** 2`.
`$powi(x, 2)` multiplies once, `1 * (x * x)`, as `__powidf2` does, so
**a variable's `powi(2)` is `x * x`**: read twice with nothing run
between, it's the same value, of NaN, ±0 and the infinities too (ADR
0262's case A). Not `x ** 2`, which JS leaves the engine to approximate.
Another power, and a square of what isn't a variable, `(a + b).powi(2)`,
stay `$powi`; an `f32`'s stays `$powiF32`, rounded to one. Held by
`test/corpus/powi_square.rs` and the mutations `powi-any-power-a-square`
and `powi-square-helper`.

## Amendment: a field's square is `p.x * p.x` (#346)

ComplexNumber, ported, read `$powi(complexDivider.re, 2)` and `$powi(this.re,
2)`. **A field's square, of a variable or of `this`, is `p.x * p.x`** too,
where reading it does nothing (`has_effects`): a getter's read, marked so
(`Known::Getter`, ADR 0672), may, and stays `$powi(window.scrollY, 2)`,
read once. Held by `test/corpus/powi_square.rs`, `lowering-numbers.test.ts`'s
"a square of a field is x * x, of a getter's read $powi" and the mutations
`square-of-a-getter-read-twice` and `square-of-a-field-helper`.
