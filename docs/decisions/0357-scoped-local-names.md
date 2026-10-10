# 0357. A local reuses a name where JS's scopes keep each read its own

Status: Accepted. Amends [0038](0038-js-names-and-destructuring.md) and
[0352](0352-reclaimed-names.md).

## Context

Every local of a function had a name of its own: `n` and `n$1` for two
`let n`s in sibling blocks, though JS lets a block's `const` shadow another:

```js
if (a) {
  const n = 1;
  return n + 1;
}
const n$1 = 2;
return n$1 * 3;
```

A person writes `const n = 2;`. The printer leans on the old rule: an
`else` after a branch that leaves is written after its `if`, in the block
around it (ADR 0237), its locals beside that block's.

## Decision

**A local `x$k` is `x`, or else the first of `x$1` on that works, where every
read in its item is of the declaration it was, as JS's scopes resolve
it**, chosen with the user:

- Each read is resolved, before and after: a block's, a closure's, a
  function's, a loop head's and a `catch`'s declarations, hoisted to the
  start of their scope (a `const` read before it's made is still its
  block's), parameters and a function body's own sharing one scope.
- A scope declaring one name twice is refused, as JS refuses it.
- An `else` after a branch that leaves is its enclosing block's, as the
  printer writes it; `js::leaves` is the one rule both use.
- What the printer writes of JS's own is read too: `String` of a
  conversion, `undefined`, `NaN`, `Infinity`.

## Consequences

- `const n = 2;` above; `result$2` beside `result` is `result$1`.
- A closure's parameter may shadow a name outside it,
  `json.flat(rest, (value, json) => ..)`, where its body reads only its own.
- The corpus and snapshots: 99 files, renames only, each still run; the
  react.dev port: 2 files, `className` and `decorator` as react.dev names
  them.

## Amendment: one scope tree, each rename checked against it

Walking the item again for each candidate name took inbox's serde code 42
seconds. An item's scopes are built once instead: each scope's
declarations, and each read's scope and its declaration's. A rename of
`from` to `to` keeps each read where no scope declaring `from` declares
`to`, no read of `from` is from outside or passes a scope declaring `to`
on its way to its own, and no read of `to` passes one declaring `from`.
The tree is renamed with the item, so each candidate is a check of its
own reads: inbox is a second again, its JS the same. The conditions no
Rust rust-js lowers reaches, as it numbers names in order, are
`src/prepare/scopes.rs`'s unit tests.
