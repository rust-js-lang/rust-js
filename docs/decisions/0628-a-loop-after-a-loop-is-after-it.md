# 0628. A loop after a loop is after it

Status: Accepted. Extends ADR 0237's guards and the tidy of loops'
tests (`while_tests`).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: where a statement
is written, not what it does.

## Context

React's `normalizeConsoleFormat` walks its format string, then pads it,
two `while` loops one after the other. MIR puts what follows a loop in
the loop's way out, and tidy moves it back after the loop
(`exits_after`), but only a way out that ends in a `return`, a `throw` or
a jump. Here it ended in the second loop, which leaves only by its own
`return`, so both stayed as MIR has them:

```js
while (true) {
  if (i >= text.length) {
    let j = 0;
    while (j < n) { .. }
    return out;
  }
  ..
}
```

## Decision

**A loop nothing `break`s ends a way out as a `return` does**: it never
goes on, so what the program wrote after the first loop, the second and
all after it, is moved after it, the way out a `break`. And **a loop
whose one statement is `if (t) { exit } else { rest }`**, its `else` not
yet taken apart, is a loop of that test, as `if (t) { exit } rest` is:

```js
while (i < text.length) {
  ..
}
let j = 0;
while (j < n) {
  ..
}
return out;
```

## Why

- **It's as the program wrote it**: two loops one after the other, no
  `while (true)`.

## Consequences

- Six corpus programs write their loops one after another, flatter, their
  temporaries declared where they're used, and with fewer bare
  `return;`, 84 to 78 in the readability count; names given a `$1`
  grow, 850.8 to 856.1 in ten thousand lines, as loops that were nested
  now share a scope, whose iterators a person names apart too.

## What holds it

- `test/lowering-loops.test.ts`, "a loop after a loop is after it".
- Mutations `unbroken-loop-goes-on`, `loop-test-of-else-unread`.
