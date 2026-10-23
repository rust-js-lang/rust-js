# 0685. A `let` a loop's body declares is in it, set in a block MIR made

Status: Accepted. Extends [0364](0364-mir-lowering.md)'s tidy, whose
`sunk_lets` declares each `let` where it's first named.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Where a variable is
declared, of one set before each read: nothing that runs changes.

## Context

React's `describeObjectForErrorMessage`, ported, declared `let substr;`
before each of its loops, where the original declares it in the loop's
body:

```js
// The original:                      // rust-js's:
for (let i = 0; ..; i++) {            let substr;
  ..                                  for (const [i, value] of ..) {
  let substr;                           ..
  if (..) {                             if (..) {
    substr = ..;                          substr = ..;
  } else ..                             } else ..
```

MIR sets `substr` in a labeled block it makes of the `if` chain, `l: { if
(a) { if (b) { substr = x; break l; } } substr = y; }`, read after the
block. `sunk_lets` moves a `let` into a loop's body only where the body
sets it before anything reads it each turn, and it knew a labeled block
only where the block is all that names the variable, and an `if` that
sets it in each branch: not a block whose every way out sets it, read
after.

## Decision

**A statement sets the variable first where each way on through it sets
it before anything reads it** (`assigned_on`), the way on of each of its
parts being what follows that part:

- an `if`: each branch does, or names nothing and what follows it does;
- a labeled block: each way out does, its end and its `break`s, a `break`
  going where what follows the block goes;
- a `while (true)`: each of its `break`s does, its end going round again;
- a way that leaves the turn, by `continue`, `return`, `throw` or the
  loop's own `break`, doesn't go on;
- a `break` of a block or a loop inside the turn, before it's set, goes on
  unset past what follows, and so doesn't, unless where it goes sets it.

A `let` such a statement sets first, in a loop's body, is in that body.
Where a labeled block or an `if` is the last that names the variable,
nothing reads it after: a branch that names nothing is fine, and the
block's `break` goes where nothing reads it, as before.

## Why

- **It's where the program declares it**, in the Rust and in React's JS.
- **The rule is the one it was, for each way through**: a way that goes
  on is checked to where it goes, as Rust's own check of a variable set
  before it's read is, not only an `if` of two branches. Where
  `assigned_on` read an `if` as all that names it, a branch that set the
  variable only sometimes counted as setting it, and a labeled `break`
  before it's set as going nowhere; neither does now.

## Alternatives

- **Every `let` with no value into the loop it's first named in**: Rust's
  checks say a variable is set before it's read each turn, but a
  variable declared before the loop may carry a value from one turn to
  the next. Rejected.

## Consequences

- Three corpus cases declare a `let` where the program does:
  `or_bindings`' `r$2`, in its loop's `if`, which frees `r` for a later
  `const`; `interpreter`'s `Parser.program`, its `let stmt;` in its loop;
  and `loop_value_after_loop`'s `tmp`. No snapshot changed; in
  `docs/readability.txt` two renames fewer.

## What holds it

- `test/lowering-loops.test.ts`, "a let a loop's body declares, set in a
  chain's block, is in the body", run.
- Mutations: `sunk-let-block-unset`, `sunk-let-if-falls-through-unset`,
  `sunk-let-own-break-unknown`, `sunk-let-loop-unset`. A `break` past what
  reads a variable before it's set has none: Rust's own check that a
  variable is set before it's read leaves no program whose JS goes there,
  so it guards the rule, not a program.
