# 0371. Tests and names as a person writes them

Status: Accepted. Extends [0364](0364-mir-lowering.md),
[0369](0369-tuple-parts-held-each.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)) for the names and the
turned comparisons, which nothing can observe; the checks left out can't
fail, so nothing runs differently either.

## Context

Three shapes of MIR's lowering read as no one writes JS:

```js
const value = v.pop();             // a tuple's part held (ADR 0369)
if (value != null) {
  if (n > 3) {
    const x = value;               // the arm's binding of it
    return x;
  }
}
if (!(1 < 4)) {                    // `grid[1][2] = 7` of a `[[u8; 3]; 4]`
  throw new Error("index out of bounds: the len is 4 but the index is 1");
}
if (!(b >= 48) || !(b <= 57)) {    // `!matches!(b, b'0'..=b'9')`
```

## Decision

- **A held part takes the name an arm binds it by**, as a temporary an
  `if let` binds already did: `const x = v.pop(); if (x != null) { ..
  return x; }`. Arms that each bind it by one name, `(n, Some(x))` and
  `(n, None)`, given apart as `n` and `n$1` until they're named in their
  scopes (ADR 0357), are that name.
- **A check that can't fail isn't written**: a comparison of two numbers
  is known, `1 < 4` of a constant index in an array's constant length,
  and a failed check of a known `true` is none.
- **`!(a < b)` of what's never `NaN` is `a >= b`**: an integer, which tidy
  knows by its variable (`Hints::numbers`), or a literal; what a bitwise
  operator or a `length` gives; or a string compared to a string literal,
  as Rust's types say the other side is a string too. Not a float's, whose
  `NaN` is neither less nor more: `!(x < 50)` of an `f64` stays.

## Consequences

- `loop_exits` runs a float's exit with a `NaN` against native Rust, and
  an integer's, turned; `tuple_scrutinee_calls` names what arms bind;
  `array_repeat` and twenty other corpus programs lose checks that can't
  fail.
- Not yet: a comparison of what tidy can't type, an `Option`'s value after
  its `== null` test, a cell's `value`, stays `!(t < 5)`.
