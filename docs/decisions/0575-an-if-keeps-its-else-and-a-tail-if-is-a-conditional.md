# 0575. An `if` keeps the `else` it's written with, and a tail `if` of two values is a conditional

Status: Accepted. Extends [0364](0364-mir-lowering.md), whose tidy writes
a body as a person writes it, and [0432](0432-a-return-is-written-where-its-value-is-made.md)'s
`continue`s; replaces the rule that a function's tail `if` returns each
branch (rust-js's form, accepted of react.dev).

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Each rewrite changes
nothing that runs.

## Context

React's scheduler heap, ported (ADR 0574), read worse than its original
in three places, each of an `if` with an `else`:

```js
// siftUp, the original:            // rust-js's:
if (compare(parent, node) > 0) {    if (!(compare(..) > 0)) {
  ..                                  return;
} else {                            }
  return;                           ..
}
// siftDown's `else if`:            } else {
} else if (rightIndex < length      if (rightIndex < length && ..) {
           && ..) {                     ..; continue;
                                      }
                                      return;
                                    }
// compare:
return diff !== 0 ? diff : a.id - b.id;     if (diff !== 0) { return diff; }
                                            return a.id - b.id;
```

MIR's structure makes a guard of an `else` that never goes on, a `return`
or a `break`, and of a branch the loop goes on from a `continue`; and it
copies the function's one return block onto each path, so a tail `if`'s
value is a `return` of each branch. tidy kept those as they came, and
`branch_values`, which makes `x = t ? a : b` of a variable's two values,
left out the function's on purpose.

## Decision

**An `if` the program wrote with an `else` keeps it** (`written_elses`):
`if (t) { exit } rest`, where `exit` never goes on and `rest` ends the
list, is `if (t) { exit } else { rest }` where `exit` is written in its
`then`, and `if (!t) { rest } else { exit }` where it's its `else`. Which
statement is of which branch is where it's written: THIR gives each such
`if`'s condition and branches (`Written::elses`). It runs once its tests
are one (`and_tests`); the `continue`s its branches then end with, which
the program didn't write, go as any at a loop's end do (`tail_continues`).
An `else` of an `if` is its `else if`.

**A tail `if` of two small values is one conditional**, `return t ? a :
b;`: of the function's value as of a variable's, one rule
(`branch_values`); and where each path returns it, of an `if` written with
an `else` (`returned_values`). Not:

- of a guard the program wrote, an `if` with no `else`, `if c { return a;
  } b`, nor a `let … else`;
- of a `return` the program wrote in each branch, a statement it wrote as
  one (`jumps::written`);
- of a two-arm `match`, which returns each arm as it did: no `if` the
  program wrote;
- where a branch gives nothing, `undefined`, whose guard, `if (t) {
  return a; }`, reads as one (`folds_into_return`);
- of a branch with statements or a comment, as `branch_values` keeps them.
- And `if (t) { return a; } return;`, a lone jump each way, keeps no
  `else`: the `return;`, which the program didn't write, goes.

**Each branch of a conditional maps to its own Rust** (`spanned`): the
printer maps a name or a literal where it starts, not a binary or a call,
so a branch's first token takes the place of the nearest of what it's in,
`fib` of `fib(n - 1)`, as the `return` statement it was mapped it.
`logical_returns`' exclusion of an `if`'s condition, which kept its
branches returned, goes with the rule it served.

## Why

- **It's how it's written**, in the Rust and in the JS a person writes of
  it: React's own heap, `if/else` and `else if` in a loop, and `compare`'s
  `diff !== 0 ? diff : ..`.
- **One rule of a value an `if` gives**: a variable's was a conditional
  and the function's was branches, by where the value went. Now it's by
  what the branches are.
- **The program's own `if`s tell**, as its conditions, `&&`s and
  `continue`s already do (`Written`): a guard it wrote stays a guard.

## Alternatives

- **Keep the guard of an `else`**: React's `siftUp` and `siftDown` as no
  one writes them, `!(c > 0)` and a `continue` the program didn't write.
  Rejected.
- **Keep each branch returned**, the form react.dev's port accepted:
  `if (c) { return 1; } return 2;` of `if c { 1 } else { 2 }`, which
  React writes, and a person does, `c ? 1 : 2`. Rejected, on the user's
  decision for React's port.
- **Make any `if (t) { return a; } return b;` a conditional**: a guard the
  program wrote isn't a value. Rejected.

## Consequences

- 20 corpus cases and 14 snapshots read shorter: 143 lines fewer. The
  readability baseline's counts don't grow; its shares of `$iter` and
  `Iterator.from` move up a tenth, of fewer lines, and the renames' count
  goes down by seven.
- Tests that held the tail `if`'s branches now hold the conditional.

## What holds it

- `test/lowering.test.ts`, "an if written with an else keeps it, in a
  loop and of an else-if": `siftDown`'s nested `if/else` and `else if`,
  `compare`'s conditional, a guard and an `undefined` branch kept; run.
- `test/lowering.test.ts`, "a tail if of two small values is one
  conditional"; "a string's parts from MIR are an array", of `return`s
  written in each branch; "a function's match returns each arm, and its if
  of two values is one conditional".
- `test/emission.test.ts`, "source map points from fib.js back into
  fib.rs": `fib((n - 1) >>> 0)`, an `else`'s value, maps to `fib(n - 1)`.
- Mutations: an `else` left a guard, each branch returned, a `match` made
  a conditional, an `undefined` branch made one, written `return`s folded,
  a lone jump given an `else`, a branch left unmapped.
