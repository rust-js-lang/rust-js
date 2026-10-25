# 0859. A `loop` that ends an `if`'s branch breaks as a loop, its `else` kept

Status: Accepted. Extends [0705](0705-a-guards-scope-reads-as-any-body.md)'s
amendment, whose loop that ends a block MIR made took the block's `break`
it ended with as its own, to each of the loop's `break`s of the block,
and to a `break 'rows` of a loop inside it.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): each `break` goes
where it went; only how the JS says so changes.

## Context

React's `mapIntoArray` (ReactChildren.js) walks an iterable's iterator in
an `if` whose `else if` follows:

```js
if (typeof iteratorFn === 'function') {
  ..
  while (!(step = iterator.next()).done) { .. }
} else if (type === 'object') {
  ..
}
```

Its port's `loop { .. if step.done { break; } .. }` printed a label and a
plain `if` where upstream's `else if` was:

```js
bb260: {
  if (typeof iteratorFn === "function") {
    while (true) {
      if (step.done) {
        break bb260;
      }
      ..
    }
  }
  if (type === "object") { .. }
}
```

MIR sends the loop's `break` to where the `if` and its `else` meet, past
the `else`; the structure writes that as a block MIR made around both,
`bb260`, and tidy kept the `if`'s `else`. But a loop whose only way out
is a `break` of the block never goes on, so preparation took the `if` for
one whose branch leaves (ADR 0237) and wrote its `else` after it. ADR
0705's amendment made the block's `break` the loop's own only where the
loop's body ended with it, not in an `if` inside the body. React's
calculator example, `to_rpn`, had the same `bb49`.

## Decision

**A loop that ends a block MIR made, through the `if`s and blocks it ends,
leaves the block by its own `break`: each `break` of the block in the
loop's body, not in a loop or a `switch` inside it, is the loop's.** The
block's label is then named by nothing and goes, and the `if` keeps its
`else`, as the program wrote it:

```rust
if kind == "iterable" {
    loop {
        if at >= items.len() {
            break;
        }
        count += items[at];
        at += 1;
    }
} else if kind == "object" {
    ..
}
```

```js
if (kind === "iterable") {
  while (true) {
    if (at >= items.length) {
      break;
    }
    count = (count + items[at]) >>> 0;
    at++;
  }
} else if (kind === "object") {
  ..
}
```

- **One of a loop inside it**, `break 'rows` in a `for` of the `loop` the
  program labeled `'rows`, is the outer loop's, by its label: the block's
  label is the loop's, where the loop has none and nothing else names the
  block, which `named labels` names as the program did, `rows: while
  (true) { for (..) { .. break rows; } }`. Where something else names the
  block, it stays as it was.
- **Once what's after each loop is placed**: the pass, `loops ending
  blocks`, runs in the `settled` stage, after `exits after` and `while
  tests` have read each loop whose way out MIR moved into it, `while (i <
  n && digit(i)) { i++; }`, as they need its `break` of the block. The
  earlier `blocks loops end` still takes the `break` a loop's body ends
  with, which `while tests` reads its test by.

## Why

- **It's the JS a person writes**: React's `else if`, and no label
  anywhere upstream has none.
- **It's the same jump**: the loop ends the block, so its `break` goes
  where the block's end is, through nothing that runs.

## Alternatives

- **Preparation keeps the `else` of an `if` whose loop leaves a block**:
  the `break bb260` stays, a label upstream doesn't have.
- **Every `break` of a block, wherever it is, the innermost loop's**: one
  in a statement after the loop, or in a loop of its own inside, goes on
  where the block wouldn't.

## What holds it

- `test/corpus/loop_ends_branch.rs`: React's shape, a `loop` ending a
  branch of an `else if` chain inside an `else`, and a `'rows: loop`
  whose `for` and `match` leave it, run beside native Rust.
- `test/snapshots/calc/calc.js`, `to_rpn` with no `bb49`; the readability
  baseline's labeled blocks one fewer, and its `while (true)` share up by
  the corpus case's two, each a `loop` the program wrote (ADR 0565).
- Mutations in `scripts/mutations/lower/mir/tidy/jumps/continues.ts`:
  `loop-ending-branch-breaks-unnested`,
  `loop-ending-branch-inner-loops-unlabeled`,
  `loop-ending-branch-label-not-moved`.
