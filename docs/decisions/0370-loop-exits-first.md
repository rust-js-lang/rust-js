# 0370. A loop's exits come first, as a person writes them

Status: Accepted. Extends [0364](0364-mir-lowering.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): where an exit is
written in a loop's body is no part of what the program can observe.

## Context

`while let Some(x) = stack.pop() { .. } sum` was already written as a
person writes it, `while (true) { const x = stack.pop(); if (x == null) {
break; } .. } return sum;`. Where the loop's end was a `return` instead, a
function that gives `None` after it or `Ok(left)`, MIR's `return` was
written where the loop ends, and its body nested in the test that goes on:

```js
while (true) {
  const part = $next(parts);
  if (part != null) {
    const value = dir.get(part);
    if (value) {
      // ..
      return;
    }
    return;
  }
  return;
}
```

The playground's compiler, walking a path, read so.

## Decision

**A `while (true)` whose body ends in an exit writes it first:**

- **`if (c) { a; continue; } return x;`**, where `a` goes on: `if (!c) {
  break; } a` and `return x;` after the loop, as the `break` form is
  written. Not where anything else leaves the loop, which would reach that
  `return` too, nor where `x` reads what the loop declares, which isn't
  seen after it.
- **`if (c) { a } return x;`**, where `a` never goes on past its own end:
  `if (!c) { return x; } a`.
- **`if (c) { a } else { e }`**, `e` a lone exit, `return;` as a `?`'s is
  or `break;`: `if (!c) { e } a`, as `a` goes on round the loop where the
  body ends.
- **A lone exit stays first.** A loop body's `if (t) { a } rest`, `a`
  written after `rest`, is put in the order written (ADR 0364); a lone
  `return;`, `break;` or `continue;` has no place of its own, a `?`'s is
  where the block it shares was written, and stays a guard.

The steps run where the loop's tests are first made, and again once
later passes have made more of them that shape, `calculator`'s.

## Consequences

- `test/corpus/loop_exits.rs` runs each case against native Rust: a path
  walk with a `?`, a `return` of what the loop declares, a test whose arm
  goes on round the loop, a name the arm shadows, and loops left by
  another `break`.
- A recursive descent parser's loop, `calculator`'s, reads `if (!(+ or
  -)) { break; } ..` and `return { TAG: "Ok", _0: left };` after it.
- Not yet: `if (!(x < 50))` stays so; tidy doesn't know `x` is no `NaN`,
  which `x >= 50` would need.
