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

## Amendment: a `let … else`'s pattern is one test, and its guard first (2026-10-31, rd299)

React's noop renderer's dumpTree takes two values apart at once:

```rust
let (Some(root), Some(root_container)) = (root, root_container) else {
    console::log(("Nothing rendered yet.",));
    return;
};
```

Its pattern was tested a part at a time, and its rest kept inside the
tests, its `else` after them:

```js
if (root != null) {
  if (rootContainer != null) {
    ..the rest..
    return;
  }
}
console.log("Nothing rendered yet.");
```

where upstream writes the guard first, `if (!root || !rootContainer) {
console.log('Nothing rendered yet.'); return; }`, then the rest, as a
`let … else` of one pattern already prints (ADR 0879, amended).

- **A `let … else`'s pattern is one condition**, as a `match` arm's
  is (`hints::written`'s `conditions`): its parts' nested `if`s are one
  `&&`, `if (root != null && rootContainer != null)`.
- **Its bindings aren't where its rest is written.** MIR binds what the
  pattern takes apart first in the rest, `const root$1 = root;` or a
  loop's `const [a, b] = pair;`, at the pattern's place, before the
  `else`; so the rest looked written before the `else`, and the `else`
  wasn't put first. `let_else_first` and `written_order_steps` now read
  where the rest is written past those bindings (`tried.rs`'s
  `let_else_bindings`): a `Const` or destructuring of a read, written
  before the `else` of the `let … else` whose `else` it is.

So the guard is first, its test De Morgan's negation of the pattern's:
`if (root == null || rootContainer == null) { console.log("Nothing
rendered yet."); return; }`, and in a loop `if (pair[0] == null ||
pair[1] == null) { ..; continue; } const [a, b] = pair; ..`. An `Option`
is tested by `== null` (tr1), not by its truth as upstream's `!root` is.
The corpus is otherwise as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests, in
the same order, short-circuited as they were.

Held by the corpus case `let_else_tuple_guard` and the mutations
`let-else-pattern-no-condition` (`mir/hints.ts`),
`let-else-bindings-taken-as-rest` and
`loop-let-else-bindings-taken-as-rest` (`mir/tidy/jumps/tried.ts`).
