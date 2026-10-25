# 0887. A std function or constant that's `Math`'s is `Math`'s as a value

Status: Accepted. Extends [0064](0064-numbers.md), whose calls are
`Math`'s, to their values; [0365](0365-js-math-where-rust-differs.md),
which left the rest of `Math` to std's own items, and
[0283](0283-js-crate-covers-typescripts-es-library.md)'s scope, which
doesn't measure it; [0602](0602-a-binding-taken-as-a-fn-is-its-function.md),
a binding taken as a `fn` being its function, to std's; and
[0597](0597-whether-the-host-has-a-global-is-is-defined.md)'s
`js::is_defined`, with [0624](0624-is-defined-of-a-path-of-bindings.md)'s
and [0645](0645-is-defined-of-a-static-is-of-the-static.md)'s, to a std
function that's `Math`'s.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each item in the table
answers as the Rust does for every value of its type, so the function or
the constant itself is the Rust one; an item that differs for some values
is left as it was, the arrow of its call, Rust's answer (case C).

## Context

React's react-reconciler/src/clz32.js is a polyfill:

```js
export const clz32: (x: number) => number = Math.clz32
  ? Math.clz32
  : clz32Fallback;

const log = Math.log;
const LN2 = Math.LN2;
function clz32Fallback(x: number): number {
  const asUint = x >>> 0;
  if (asUint === 0) {
    return 32;
  }
  return (31 - ((log(asUint) / LN2) | 0)) | 0;
}
```

Its port is the Rust an engineer writes of it, std's `u32::leading_zeros`,
`f64::ln` and `std::f64::consts::LN_2`:

```rust
thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

const LOG: fn(f64) -> f64 = f64::ln;
const LN2: f64 = std::f64::consts::LN_2;
fn clz32_fallback(x: u32) -> u32 {
    let as_uint = x;
    if as_uint == 0 {
        return 32;
    }
    31 - (LOG(as_uint as f64) / LN2) as u32
}
```

`js::is_defined(u32::leading_zeros)` was refused, of what isn't a JS
global's binding. `f64::ln` taken as a `fn` was an arrow, `(n) =>
Math.log(n)`, and `LN_2` its number, `0.6931471805599453`. ADR 0365 says
`Math` is Rust's own `f64` and `u32`, and binds in `js::math` only what
answers otherwise; so `u32::leading_zeros` is the program's `Math.clz32`,
and nothing let it say so as a value.

## Decision

The user's decision: **a std item whose JS is a `Math` member is that
member when it's used as a value.** A std function taken as a `fn` is
`Math`'s function, `js::is_defined` of it asks whether the host has it,
and a constant of `std::f64::consts` is `Math`'s constant:

```js
const { log: LOG, LN2 } = Math;

export const clz32 = typeof Math.clz32 !== "undefined" ? Math.clz32 : clz32Fallback;
```

**Only where `Math`'s answers the Rust one for every value of its type.**
One table holds which, `src/lower/std_types/number/math.rs`, and the call,
the value, `js::is_defined` and a constant's read all ask it:

| Rust | `Math` | Why it's exact |
|---|---|---|
| `u32::leading_zeros`, `i32::leading_zeros` (`usize`, `isize`) | `clz32` | `Math.clz32` reads its number's 32 bits, an `i32`'s sign among them: `-1`'s is `0`, as Rust's is |
| `i8`, `i16`, `i32::unsigned_abs` | `abs` | `i32::MIN`'s is 2^31, which a `u32` holds |
| `i8`, `i16`, `i32::signum` | `sign` | an integer is never JS's `-0` (ADR 0064) |
| `f64::floor`, `ceil`, `trunc`, `abs`, `sqrt` | `floor`, `ceil`, `trunc`, `abs`, `sqrt` | IEEE 754's exact operations, `-0.0` and `NaN` kept |
| `f64::cbrt`, `exp`, `exp_m1`, `ln`, `ln_1p`, `log10`, `log2`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `hypot`, `atan2` | `cbrt`, `exp`, `expm1`, `log`, `log1p`, `log10`, `log2`, .., `hypot`, `atan2` | what the call is already (ADR 0064): a library function's last bit is the platform's, JS's or C's, either way (ADR 0064's Consequences); the value is no further from Rust's than the call |
| `f32::floor`, `ceil`, `trunc`, `abs` | `floor`, `ceil`, `trunc`, `abs` | exact of an `f32`, so they're `f32`s (ADR 0122) |
| `std::f64::consts::E`, `LN_2`, `LN_10`, `LOG2_E`, `LOG10_E`, `PI`, `SQRT_2`, `FRAC_1_SQRT_2` | `E`, `LN2`, `LN10`, `LOG2E`, `LOG10E`, `PI`, `SQRT2`, `SQRT1_2` | the same `f64`, bit for bit, as the corpus prints them |

**Not in the table, as they were:**

- `f64::round`: `Math.round` takes a half up, `-2.5` to `-2`, where Rust's
  takes it away from zero. Taken as a `fn`, it's `(n) => $round(n)`;
  `js::math::round` is `Math.round` (ADR 0365).
- `f64::signum`: `Math.sign(0)` is `0`, Rust's `1.0`; `f64::max` and
  `min`: `Math.max` of a `NaN` is `NaN`, Rust's the other.
- An `f32`'s rounded functions, its `sqrt` and `sin`: each result is
  rounded to an `f32`, `(n) => Math.fround(Math.sqrt(n))`.
- A `u8`'s or `u16`'s `leading_zeros`: `Math.clz32(n) - 24`, as its call.
- `exp2`, `powf`, `powi`, `log(base)`: not one `Math` function of the
  method's arguments.
- `i32::abs`: `Math.abs(n) | 0`, `i32::MIN`'s wrapped; any 64-bit or
  128-bit integer's, a BigInt, which `Math` doesn't take.
- `std::f32::consts`: `f32`s, not `Math`'s `f64`s; `TAU`, `FRAC_PI_2` and
  the rest, which `Math` hasn't.

**What a value is, where:**

- **A function taken as a `fn` pointer is `Math`'s function, read where
  it's taken**, as a binding's is (ADR 0602): `const LOG = Math.log`, a
  pointer chosen at load, `Math.clz32`, which a polyfill set later doesn't
  move. A function item handed to an iterator is its arrow, as a binding's
  is, `.map(f64::sqrt)` `.map((n) => Math.sqrt(n))`; and what hands a
  `fn` pointer to JS gives it no more than Rust does, as it did,
  `[3, 4].reduce((a, b) => hypot(a, b), 0)`, so `Math.hypot` isn't given
  an index.
- **`js::is_defined` of a std function in the table asks of its function,
  `typeof Math.clz32 !== "undefined"`.** Not of `Math` itself: it's
  ECMAScript's, a property of every global object, where ADR 0624 asks
  each part of a path because a host may lack any. One not in the table,
  `js::is_defined(f64::round)`, is refused as it was, as what isn't a
  global's.
- **A constant is `Math`'s constant where it's read, `Math.PI * r * r`,
  and where a `const` is written as one, `const LN2 = Math.LN2`.** A
  `const` written as a number, `const PI: f64 = 3.141592653589793`, or as
  a computation, `PI / 2.0`, is the number rustc computed, as it was.

## Why

- **It's the JS a person writes**: `const log = Math.log`, `Math.PI *
  r * r`, `typeof Math.clz32`, of the Rust a person writes, with nothing of
  rust-js's own.
- **One table, one answer**: whether `Math.f` is the Rust function is
  decided once, and the call (`Math.sqrt(x)`), the value (`Math.sqrt`)
  and `js::is_defined` can't disagree. The `f32` rule of which `Math`
  calls are exact, written twice before (the call's and the value's), is
  the table's.
- **No result changes**: the table holds only what's exact, or what the
  call was already.

## Alternatives

- **Every `Math` call's function as its value, `Math.round` too**: a
  different answer at a negative half (case C refuses it).
- **`Math.f` as a function item handed to JS, `.map(Math.sqrt)`**: JS
  gives a callback more than Rust does, `Math.hypot` an index and the
  array; ADR 0602 keeps an item's arrow for this reason.
- **`js::math::clz32` and its like, bound**: two spellings of one
  function, std's and the js crate's, which ADR 0365 rejects.
- **`typeof Math !== "undefined" && typeof Math.clz32 !== "undefined"`**:
  asks of what ECMAScript guarantees.

## Consequences

- **`std::f64::consts::PI` reads `Math.PI` everywhere it's used, where it
  read `3.141592653589793`**: intended, as the JS a person writes. Of the
  corpus, `calculator.js` (`vars.set("pi", Math.PI)`), `mul_add.js`
  (`Math.E`) and `number_methods.js` (`Math.PI * (180 / Math.PI)`); of the
  guide, four examples of a circle's area, `Math.PI * radius * radius`
  (basics, advanced, objects, types). A literal the Rust wrote stays the
  literal.
- **`docs/readability.txt` blessed**: each share shrinks, the corpus case
  new JS of 154 lines whose two renames, `value$1`, are of a `{:?}`'s
  values, as before.
- **The port's JS against upstream's**, line by line:

  | upstream | rust-js |
  |---|---|
  | `export const clz32 = Math.clz32 ? Math.clz32 : clz32Fallback;` | `export const clz32 = typeof Math.clz32 !== "undefined" ? Math.clz32 : clz32Fallback;`: `js::is_defined`'s `typeof`, where upstream tests its truth (ADR 0645's alternative) |
  | `const log = Math.log;` `const LN2 = Math.LN2;` | `const { log: LOG, LN2 } = Math;`: two constants side by side reading one object are destructured (`prepare.rs`'s `read_together`); `LOG`, Rust's name of a `const` |
  | `const asUint = x >>> 0;` | `const asUint = x;`: a `u32` is one already |
  | `return (31 - ((log(asUint) / LN2) \| 0)) \| 0;` | `return (31 - $f64ToInt(LOG(asUint) / LN2, 0, 4294967295)) >>> 0;`: Rust's `as u32` saturates, `NaN` to `0`, where `\| 0` wraps; the value comes from a `fn` pointer, so no range is known of it (ADR 0881's ranges are integers'), and `- ` of a `u32` wraps as release Rust's |

  rust-js writes the constants before `clz32`, where upstream writes them
  after it; `pub static` in `thread_local!`, never set, is an `export
  const` (ADR 0270's box is of one that's set).

## What holds it

- `test/lowering-math.test.ts`: the port as written, its JS, and run with
  `Math.clz32` and without it, its fallback chosen; an `f64::round`, an
  `f32::sqrt` and a `u8::leading_zeros` taken as a `fn`, each its arrow,
  a literal `const` its number, a function item's arrow; and
  `js::is_defined(f64::round)` refused.
- Corpus `math_values` against native Rust: the polyfill both ways over
  every kind of `u32`; each function in the table taken as a `fn` and
  called, `-0.0`, `NaN` and infinities among their arguments; those not
  in it; a pointer handed to `fold` and `map`; each constant, to its bits,
  and those written as a literal and a computation.
- The guide's [Numbers](../guide/interop/types.md#numbers), [a global some
  hosts lack](../guide/interop/global-scope.md#a-global-some-hosts-lack)
  and the js crate's `js::math`.
- Mutations: `math-f32-unrounded`, `math-narrow-clz32`,
  `math-float-signum`, `math-constant-other-member` and
  `math-written-constant-any` (`lower/std_types/number/math.ts`);
  `math-call-unasked` (`number.ts`), `math-pointer-arrow` (`mir/rvalues.ts`),
  `math-defined-refused` (`mir/calls.ts`), `math-constant-literal`
  (`lower.ts`), `math-written-constant-literal` (`pipeline.ts`), and
  `f32-fn-value-unrounded` (`items.ts`), moved to the table.
