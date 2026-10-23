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
"Num")`, as an arm's pattern is one. A part a later part implies drops
out (`branches::unimplied`): `x != null` where `typeof x === "number"`, of
any kind but `"object"`, which `null` is, and `"undefined"`, as React's
`Object.is` polyfill has `typeof x === 'number' && typeof y === 'number'`. A drop
flag's test joined with a value's, `if (d && d$0$live)`, is still the
flag's, named for what it guards (`flags::tests_flag`).

## What holds it

- `test/lowering.test.ts`, "a match arm's pattern of parts is one test of
  each"; eight corpus cases, `interpreter`'s operators among them, and the
  `enums` snapshot.
- Mutations: the tests nested, a joined flag left unnamed, an implied
  part kept, `"object"` taken to imply a value.

## Amendment: a range's two bounds are one test

A range pattern, `90..=100`, is two tests MIR makes, which were an `if`
in an `if`: `if (score >= 90) { if (score <= 100) .. }`. An arm's pattern
is a condition the program wrote, as its scrutinee is, so its tests are
one: `if (score >= 90 && score <= 100)`, and a slice's length and its
first element alike. Held by `test/lowering-numbers.test.ts`, "a range
pattern's bounds are one test", and the mutation `range-arm-tests-apart`.
