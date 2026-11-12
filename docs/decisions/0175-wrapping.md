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

## Amendment: an `op=` through a `&mut` writes what it points at (#309)

`w += x` assigned the `Wrapping`'s place, `w = [(w[0] + x[0]) >>> 0]`, and
took a variable holding a `&mut` to it for the place. A parameter's, `fn
bump(r: &mut Wrapping<u32>) { *r += Wrapping(1); }`, was `r = [..]`, so the
caller's never changed; and MIR's borrow taken before an operand that's a
call, `b0 += b1 << 4`'s, `const _18 = b0; _18 = [..]`, which tidy made an
assignment of a `const`: TheAlgorithms/rust's TEA cipher threw as its
module loaded. **Through a variable that's a `&mut`, it's written as any
`*r = v` is**, `$assign(r, [..])`, the array the `Wrapping` is changed in
place; of the place itself, as before. Held by the corpus case
`wrapping_assigned_through_a_ref` and the mutation
`wrapping-assigned-to-its-ref`.

MIR's borrow before a call still reads `$assign(b0, [..])` where the same
`op=` without one reads `b0 = [..]`: #310.


## Amendment: `op=` of a call's operand rebinds the variable (#396, #310)

`b0 += b1 << 4` of a variable `b0: Wrapping<u32>` that a copy was made
of, `let w = b0`, changed the copy too: Rust's `1 (1, 17)` was `17 (17,
17)`. With a call's operand, MIR takes `&mut b0` into a temporary, `_5 =
&mut _1`. The rule writing through a `&mut` asked whether any local of
that name was a reference, so it wrote `$assign(b0, ..)`, into the array
the copy `const w = b0` shares.

Now a temporary of MIR's own that borrows a variable of the program's
whole is that variable, rebound, `b0 = [..]`, as it is without a call's
operand. Only a variable the program wrote that holds a `&mut`, `r` of
`fn bump(r: &mut W<u32>)`, writes what it refers to in place.

- `test/corpus/wrapping_assign_of_a_copy.rs` checks the copy and the
  caller's value against native Rust.
- `wrapping_assigned_through_a_ref.js` now reads `b0 = [..]`, where it
  was `$assign(b0, ..)`.
- The mutation is `wrapping-op-assign-written-through`.

## Amendment (2026-11-12, #1439, #1440): each operator as std's impl writes it

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is kept. Two wrong answers:

- **An `op=` by its number read it as a `Wrapping` (#1439, P0).** std has
  `AddAssign<T> for Wrapping<T>`, and so for each operator, `w += 1u8`; the
  right side was read as `[x]`, `w = [(w[0] + 10[0]) & 255]`, `undefined`,
  so `Wrapping(250u8) += 10` was `0` where Rust's is `4`. The right side is
  now read as `[x][0]` only where its type is a `Wrapping`: another
  `Wrapping`'s, not its number's, nor a shift's `usize`.
- **A signed one's `/` and `%` panicked at `MIN / -1` (#1440, P1).** std's
  impls are `wrapping_div` and `wrapping_rem`, `MIN` and `0`; rust-js wrote
  the number's own operator, which panics there. By zero, each panics as
  std's does.

**Each operator is the method of its number std's impl calls**, one table,
`wrapping_method` (recognition/wrappers.rs): `wrapping_add`, `_sub`, `_mul`,
`_shl` and `_shr`; a signed number's `wrapping_div` and `wrapping_rem`,
`$wrappingDiv(a[0], b[0], -128)`; `wrapping_neg`. An unsigned one's `/` and
`%` are its number's, which those methods are, `$div(a[0], b[0]) & 255` as
before; a bit operation and `!` are the number's own. `wrapping_result`
(std_types/number.rs) lowers each by `number_call`, the method's own
lowering, for an operator, an `op=`, and generic code's dictionary alike.

Held by corpus case `wrapping_ops_by_its_number_and_past_its_range.rs`:
every operator and every `op=`, by a `Wrapping` and by its number, at each
width from `u8` to `i128`, past each bound, by zero, `MIN / -1` and `MIN %
-1`, and generic code's `/` and `%`, against native Rust in both builds
(`usize` and `isize` left out, a target's width). Mutations
`wrapping-signed-div-plain`, `wrapping-signed-rem-plain`,
`wrapping-unsigned-div-a-helper`, `wrapping-rhs-always-indexed`,
`wrapping-dictionary-rhs-unread`.

## Amendment (2026-11-12, #1434): `Saturating` is a `Wrapping` whose methods saturate

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a refusal lifted.
`std::num::Saturating<T>` was refused wherever it was used: `Saturating(250u8)
+ Saturating(10)` as "calling `std::ops::Add::add`", and as a hash set's key.
It's a transparent tuple struct with derived `PartialEq`, `Ord`, `Hash` and
`Default` and the same operator impls as a `Wrapping`, so it takes the same
form: **`[x]`, each operator the method of its number std's impl calls.**
One recognizer, `int_newtype` (recognition/wrappers.rs), answers both,
`Wrapping` or `Saturating`, for operators, `op=`, `{}`, `{:?}`, order, `==`
and dictionaries, in place of `wrapping_of`; one table, `newtype_method`,
gives each kind's method:

| | `Wrapping` | `Saturating` |
|---|---|---|
| `+` `-` `*` | `wrapping_*` | `saturating_*` |
| `/` of a signed one | `wrapping_div` | `saturating_div`, `MIN / -1` its `MAX` |
| `%` of a signed one | `wrapping_rem` | the number's, which panics at `MIN % -1` as std's |
| unary `-` | `wrapping_neg` | `saturating_neg` |
| `<<` `>>` | `wrapping_shl`, `_shr` | none in std |
| an unsigned one's `/` and `%`, bit operations, `!` | the number's | the number's |

A `Saturating`'s own methods, stable where a `Wrapping`'s aren't, are its
number's, `newtype_inherent`: `pow`'s `saturating_pow`, `abs`'s
`saturating_abs`, and `count_ones`, `rotate_left`, `to_be` and the rest of
the same name, a result of its type its `[x]`. Its `MIN`, `MAX` and `BITS`
are constants rustc evaluates.

With it, for both: `{:x}`, `{:o}` and `{:b}` are the number's (they were
refused); and the derived `Default` of a std struct that's its fields in
JS, `Wrapping`, `Saturating` and `Reverse`, is its fields', `[0]` (it was
refused). `sum()` and `product()` of either stay refused, a guide
limitation.

Held by corpus case `saturating_ops_at_every_width.rs` against native
wasm32 (`//@ native: wasm32`), so `usize` and `isize` are compared as
rust-js's are: every operator and `op=`, by a `Saturating` and by its
number, at each width, past each bound, its methods, its constants, `{}`,
`{:?}`, hex, order, `Hash` as a set's key, `Default`, generic code's `+` and
`*`, and a `Wrapping` of a `usize` and an `isize`. And `saturating_ops_that_panic.rs`,
on the host, whose panics unwind: `/` and `%` by zero and a signed `MIN % -1`.
Both builds. Mutations `saturating-unrecognized`,
`saturating-arithmetic-plain`, `saturating-signed-div-plain`,
`saturating-neg-plain`, `saturating-pow-plain`, `saturating-abs-plain`,
`saturating-methods-unrecognized`, `newtype-method-result-always-wrapped`,
`saturating-not-its-fields`, `newtype-radix-refused`,
`std-fields-struct-default-refused`. #1289's refusals that named #1434 are
gone: `open_call` no longer needs a call's generic arguments, and the map's
refusal is a `Pin`'s (#1435) alone.
