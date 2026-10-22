# 0614. A match arm's pattern of parts is one test of each

Status: Accepted. Extends tidy's `and_tests`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same tests, in
the same order, short-circuited as they were.

## Context

MIR tests a pattern of several parts one part at a time, so `match (p, q)
{ (Some(x), Some(y)) => .., _ => .. }` was

```js
if (p != null) {
  if (q != null) {
```

where a JS developer writes one condition, `if (p != null && q != null)`,
as the arm is one pattern. Tidy joins nested tests where they're one
condition the program wrote, an `if`'s or a guard's.

## Decision

**A `match`'s tests, from `match` to its scrutinee's end, where MIR tests
it, are one condition** (`hints::written`): nested `if`s of them, each with
nothing else and no `else`, are one `&&`, `if (l.TAG === "Num" && r.TAG ===
"Num")`, as an arm's pattern is one. A test a later part implies drops out
as before, `typeof name === "string"` of `name != null && ..`. A drop
flag's test joined with a value's, `if (d && d$0$live)`, is still the
flag's, named for what it guards (`flags::tests_flag`).

## What holds it

- `test/lowering.test.ts`, "a match arm's pattern of parts is one test of
  each"; eight corpus cases, `interpreter`'s operators among them, and the
  `enums` snapshot.
- Mutations: the tests nested, a joined flag left unnamed.
