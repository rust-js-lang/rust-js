# Confirmed rust-js miscompiles

Both programs below are valid Rust. `rustc` and rust-js both accept them.
The JavaScript rust-js emits does not do what native rustc does.

Native builds used `rustc --edition=2024 -Coverflow-checks=off` (overflow
wraps, which is also what rust-js does). rust-js was this tree's debug
compiler, `target/debug/rust-js`, invoked as
`rust-js <file>.rs -o <file>.js -- --edition=2024`. The JS was run with
Node 22. Outputs below are stdout. Native `catch_unwind` also prints the
default panic hook on stderr; that hook text is not the bug.

Other areas that were compared and matched, so they are not reported:
integer and float casts, shifts across widths, wrapping and Euclidean
division, parsing, UTF-8 lossy decoding, `transmute` of integers to bytes,
iterator adapters, `match` guards, closure clone, assignment, formatting,
`VecDeque`, and 120 generated programs (seeds 80–119 and 500–579).

## 1. Dropping an aggregate stops after the first panicking destructor

**Severity:** severe. Destructors after the one that panics are skipped, so
cleanup does not run. This is every struct, tuple, enum variant, array,
`Vec`, and map whose parts are dropped as a group, and it is also a type's
own `Drop::drop` followed by its fields.

**Area (hypothesis):** `src/lower/drops/parts.rs`. `drop_parts` writes each
field as the next statement, and `drop_items` writes a `for...of` over
array and `Vec` elements. A panic is a thrown `Error`, so the rest of that
sequence never runs. Locals are unaffected: their cleanups are separate
unwind paths, and those still run. The generated JS for the reproducer places the second field's `println`
after `throw`, inside the callback `$catchUnwind` runs under `try/catch`
(`src/runtime/unwind.js`). That statement is unreachable:

```js
const caught = $catchUnwind(() => {
  const _pair = { second: [1] };
  console.log("boom");
  throw new Error("boom");
  console.log(`drop ${_pair.second[0]}`);
});
```

**Reproducer:** [`drop-after-panic.rs`](drop-after-panic.rs)

**Native stdout:**

```
boom
drop 1
caught true
```

**rust-js stdout:**

```
boom
caught true
```

`Loud`'s destructor does not run. The same skip shows up for a tuple
`(Loud, Boom, Loud)`, an enum variant with three fields, an array or `Vec`
whose middle element's `drop` panics (the later elements are not dropped),
a `BTreeMap` dropped in key order, and a struct whose own `Drop::drop`
panics before its fields are dropped. A field initializer that panics
during construction still drops the fields already built; that path is fine.

## 2. `f64::mul_add` returns `NaN` when an overflowing product meets the opposite infinity

**Severity:** high. The value is wrong (`NaN` instead of `±inf`) for a
defined `f64` operation. It is not a refusal and not a last-bit `Math.sin`
difference. Finite `mul_add`, and an overflowing product added to a finite
number or to an infinity of the same sign, matched native rustc.

**Area (hypothesis):** `src/runtime/fused.js`, `$fused`, which
`$mulAdd` calls. If any argument is non-finite it returns the unfused
`a * b + c`. When `a` and `b` are finite but `a * b` overflows to infinity
and `c` is the opposite infinity, that expression is `NaN`. Rust's
`mul_add` adds the exact finite product to that infinity and returns the
infinity (IEEE-754 fused multiply-add).

**Reproducer:** [`f64-mul-add.rs`](f64-mul-add.rs)

**Native stdout:**

```
-inf
inf
inf
inf
```

**rust-js stdout:**

```
NaN
NaN
NaN
inf
```

The last line is `f64::MAX.mul_add(2.0, -1.0)`, which overflows to `+inf`
on both sides. The first three lines are the bug.
