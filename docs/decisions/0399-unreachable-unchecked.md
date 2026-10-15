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
