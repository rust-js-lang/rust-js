# 0399. `hint::unreachable_unchecked()` is unreachable code's throw

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a program can't
observe it. Reaching `unreachable_unchecked()` is undefined behaviour, so
a program that's right never does, and what JS does there is JS's.

## Context

`unsafe { std::hint::unreachable_unchecked() }` tells the optimizer an arm
can't run: `match c { b'0'..=b'9' => .., _ => unreachable_unchecked() }`.
rust-js refused the call, and rustc's `for-loop-while/issue-69841.rs`
failed on it.

## Decision

**`hint::unreachable_unchecked()` is what MIR's unreachable code is,
`unreachable_code`'s one throw: `throw new Error("internal error: entered
unreachable code")`.**

```js
function digit(c) {
  if (c < 48 || c > 57) {
    throw new Error("internal error: entered unreachable code");
  }
  return (c - 48) & 255;
}
```

- **One form**: the hint and `unreachable!()`'s arm read the same; a
  throw, where nothing would, says where a wrong program went.

## Why

- **It's tested**: the corpus's `unreachable_hint.rs` runs with native
  Rust; the rustc test passes. A mutation leaves the throw out.

## Amendment: `assert_unchecked` and `unchecked_add`

Two more promises a program that's right keeps, as deranged's ranged
integers make them, were refused. Case N
([0262](0262-when-rust-and-js-disagree.md)): a program that keeps them
can't tell; one that breaks them is undefined behaviour, which Rust's
release build doesn't check either.

- **`hint::assert_unchecked(c)`** is nothing: its `c` is made as it's
  given, so `assert_unchecked(loud(x) <= 9)` still calls `loud`, and a
  condition of no effect, `MIN <= value && value <= MAX`, is gone.
- **`unchecked_add`, `_sub` and `_mul`** are what `a + b` is, `wrapping_*`'s
  form: in range, the same number.
- The `assert_unchecked` and `unchecked_and_saturating_neg` corpus cases
  run against native Rust.
