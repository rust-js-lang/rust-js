# 0599. A result its branches make before it's coerced is returned where made

Status: Accepted. Extends tidy's returns in branches.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same value
returned.

## Context

A function whose branches each make its value returns it from each,
`if (c) { return a; } return b;`. Of a `Box<dyn Fn()>`, MIR makes each
branch's box in a temporary, then coerces it into the result after the
`if`, so the function was

```js
let tmp;
if (immediate) {
  tmp = () => { .. };
} else {
  ..
  tmp = () => { .. };
}
return tmp;
```

React's scheduler picks its host callback so.

## Decision

**A temporary the branches make and the function then returns is
returned where it's made**, as the function's own result is: `if (now) {
return () => 1; } return () => n;`. A variable the program names, `let
item;` assigned in each branch and returned, stays as written.

## What holds it

- `test/lowering.test.ts`, "branches' values coerced to the result are
  returned where made"; "a `let` from MIR is declared where it's first
  named" holds the program's own.
- Mutation: of any variable.
