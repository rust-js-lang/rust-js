# 0011. Integers are JS numbers, wrapped like release Rust

Status: Accepted. Extended by [0086](0086-64-bit-integers.md): `i64` and `u64` are BigInts, and an `f64` cast to an integer saturates; by [0895](0895-an-operand-of-a-bitwise-operator-needs-no-wrap.md): no wrap where JS's bitwise operator makes it.

Case: C, D, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

JS has one number type: a 64-bit float (f64). Rust has `i8` through `u128`,
each with fixed-width wraparound. We need Rust's answers from JS numbers.

The key fact: **an f64 holds every integer up to 2^53 exactly.** For two
32-bit values, sums and differences are far smaller than that, so JS computes
them *exactly*. All we have to do is fold the exact answer back into the
type's range, which is what wrapping means.

## Decision

**Supported types:** `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `f64`, `bool`.
Anything else (`i64`, `u64`, `usize`, `f32`, `char`, ...) is a
compile error for now.

**Overflow wraps**, matching Rust's release profile (`overflow-checks = off`).

**The wrap table**, meaning how an exact result goes back into range:

| Type | Wrap | Why it works |
|---|---|---|
| `i32` | `x \| 0` | JS bitwise ops convert to signed 32-bit (ToInt32) |
| `u32` | `x >>> 0` | `>>>` converts to unsigned 32-bit (ToUint32) |
| `i8`, `i16` | `x << 24 >> 24`, `x << 16 >> 16` | shift up, then arithmetic-shift down to sign-extend |
| `u8`, `u16` | `x & 255`, `x & 65535` | keep the low bits |
| `f64` | none | JS numbers *are* f64 |

**Per operation:**

| Op | Integers | Notes |
|---|---|---|
| `+ -` | wrap(`a op b`) | exact, then wrap |
| `*` (32-bit) | `Math.imul(a, b)` (u32: `>>> 0` after) | a 32×32 product can reach 2^64, past 2^53, so a plain `*` would lose low bits. `imul` doesn't. |
| `*` (8/16-bit) | wrap(`a * b`) | product < 2^32, exact |
| `/` | wrap(`a / b`) | float division then truncation. For 32-bit operands the float quotient never rounds across an integer, so truncation gives Rust's answer. Zero/overflow checks: [0012](0012-panics-and-runtime-helpers.md) |
| `%` | `a % b` | JS `%` on integers is exact, and its sign follows the dividend, like Rust |
| `& \| ^` | as-is; u32: `>>> 0` after | JS returns *signed* 32-bit results, which only u32 must fix |
| `<<` | wrap(`a << b`); 8/16-bit: `b & 7` / `b & 15` | release Rust masks the shift amount to the type's width. JS masks to 5 bits, which is only right for 32-bit types |
| `>>` | signed `>>`, unsigned `>>>` (masked the same way) | arithmetic vs logical shift |
| `== != < ...` | `=== !== < ...` | u32 values are stored unsigned, so comparisons just work |
| `-x` | wrap(`-x`) | `-i32::MIN` wraps to itself, as in Rust |
| `!x` | `~x`; unsigned: wrap(`~x`) | |

**`bool`** is a JS boolean. `&` and `|` on bools become `!!(a & b)` and
`!!(a | b)` (both sides always evaluated, result is a boolean), and `^`
becomes `a !== b`.

**Casts (`as`):**

- int → int: nothing if the value always fits (`u8 as u32`), otherwise wrap
  to the target (`x as u8` → `x & 255`).
- int → f64: nothing, since every supported integer is exact in f64.
- bool → int: `b ? 1 : 0`.
- f64 → int: **a compile error for now.** Rust saturates (`300.0 as u8` is
  255, NaN is 0), while `| 0` is modular, and we won't ship the wrong one.

**Literals** are stored as their exact `f64` value. Every supported integer
is exact in an f64, so nothing is lost. Whole numbers print in plain decimal,
as written (`1000`, not oxc's shortest form `1e3`), and fractions print in
their shortest round-trip form ([0018](0018-print-with-oxc.md)). Negative
literals get parentheses where precedence needs them.

## Why release semantics?

Debug Rust panics on overflow. Release Rust wraps. We chose wrapping for now:

- it's what shipped Rust code does, and it's cheap and readable in JS;
- modeling debug panics means a check around almost every `+`, which hurts
  both size and readability.

A future `--overflow-checks` flag could emit checked helpers.

## Alternatives

- **Everything via `BigInt`**: exact for all widths, but slow, and it can't
  mix with plain JS numbers. We'll likely use it (or a Scala.js-style
  `RuntimeLong`) for 64-bit types later.
- **Typed arrays** for wrapping (`Int32Array` stores): correct but obscure.

## Consequences

- **This is a documented behavior difference from debug Rust:** `u32::MAX + 1`
  is `0` in rust-js, and a panic in a debug build.
- `usize` isn't supported yet. It's 64-bit on the host, and deciding how it
  maps is a separate decision.
- `usize` and `isize` were added later as 32-bit integers, as on `wasm32`
  (ADR 0025).
- `i64` and `u64` were added later as BigInts, and `f64` → int casts as
  Rust's saturating ones (ADR 0086).

## Amendment: a multiply by a small constant is JS's own `*`

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the answer is Rust's
either way; only the JS reads better.

**A 32-bit `*` with a constant of at most 2^21 on either side is JS's own
`*`, wrapped as `+`'s is**: `n * 2` is `(n * 2) | 0`, and of a `u32`,
`(n * 2) >>> 0`. Any 32-bit value is under 2^32, so its product with such a
constant is under 2^53, where a double holds every integer exactly, and the
wrap gives Rust's answer as it does for `+`. `Math.imul` stays for a product
of two variables, and for a constant past 2^21, whose product a double
would round: `4294967295 * 4194305` rounds to `…992` where Rust's wrapped
answer is `…991`. The constant is a literal or a named `const` given to the
operator, which rustc knows (`known`, ADR 0031); a method's,
`n.wrapping_mul(SCALE)`, shows only the name, and stays `Math.imul`, as
`wrapping_div` of one stays a call.

Made so, a product is a plain expression: the constant folder folds
`2147483647 * 2` to `-2`, and one made in place isn't held in a temporary
as a call is. `test/corpus/multiply_by_a_constant.rs` multiplies at each
type's edges, by `2^21` and just past it, a constant on the left, and a
named one, against native Rust; mutations `mul-by-constant-imul`,
`mul-exact-factor-too-large` and `mul-constant-on-the-left-unseen`.

## Amendment: a `bool`'s operators of a reference (#319)

A number's operator given a reference, `total += &x`, calls std's
forwarding impl, which rust-js reads as the operator of values. A `bool`'s
were refused: TheAlgorithms/rust's SHA-3, `*s ^= c` of a `&bool` `c`. **A
`bool`'s `&`, `|` and `^`, and their `op=`, of a reference are the same as
of values**: `s.value = s.value !== c`, `!!(a & b)`. Held by the corpus case
`bool_ops_by_ref` and the mutations `bool-operator-by-ref-unknown` and
`bool-assign-by-ref-refused`. Its `state.iter_mut().zip(chunk)` reads
`$lazyZip($iter($mutItems(state)), chunk)`, which grew
`docs/readability.txt`'s `$iter` share: #320.

## Amendment: `min_value()` and `max_value()` (#327)

An integer's `min_value()` and `max_value()`, deprecated for `MIN` and
`MAX`, were refused where MIR keeps the call, `Self::min_value()` in an
impl's method, and given as a value: num-integer's gcd calls them, so
rust-js refused num-integer, and num-bigint after it. **They're the
constants their bodies give**, `-128`, `65535n`. Held by the lowering test
"an integer's min_value() and max_value() are its MIN and MAX" and the
mutations `min-value-unknown` and `max-value-the-min`.

