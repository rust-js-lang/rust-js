# 0237. An `if` whose branch leaves has no `else`

Status: Accepted.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust's `if c { a } else { b }` is one expression, and rust-js wrote both
branches alike, so a function's last `if` returned in each:

```js
if (isValidElement(child) && child.type.mdxName === "inlineCode") {
  return cloneElement(child, { isLink: true });
} else {
  return child;
}
```

react.dev's own code writes the `return` after the `if`, 88 times to the
`else`'s 12 in its source, as its Link does: once the branch has left,
what's after it runs only when it didn't. ESLint's `no-else-return` says so.

## Decision

**An `if` whose branch always leaves, by a `return` or a `throw` last, or
an `if` both of whose branches do, has no `else`: what was in it follows
the `if`.**

```js
if (isValidElement(child) && child.type.mdxName === "inlineCode") {
  return cloneElement(child, { isLink: true });
}
return child;
```

- **An `else if` chain is each `if` after the last**, while each leaves.
- **A branch that doesn't leave keeps its `else`**: `if (n > 0) { total
  += n; } else { total -= n; }`.
- **Its locals stay apart**, now in the block the `if` is in: they're
  named once it's there, as the block's (ADR 0357).
- **It's how the JS is written, not what rust-js lowers**: preparation
  makes it, before its locals are named, and the printer prints what it's
  given. (Amended: the printer wrote it so, and preparation's scopes
  modelled what the printer would do to name the locals beside it.)

## Why

- **It's the JS a person writes.**
- **It runs the same**: after a branch that leaves, nothing that follows
  runs, and nothing else did; inside a `try`, a `return` still runs its
  `finally`.
- **It's tested**: a compiler test writes and runs an `else if` chain that
  returns, a branch that leaves by an inner `if`, and one that doesn't;
  the corpus runs every changed program beside native Rust.

## Since

- **An `else` of nothing is no `else`**: one of only comments in Rust,
  which rust-js doesn't carry, as react.dev's _app has, is left out.
- **A branch that ends by a `break` or a `continue` has no `else`
  either**, nor one whose last statement is a `while (true)` nothing
  breaks: `if (item === undefined) { break; } const cell = item;`, as a
  person writes a loop. Whether a statement goes on to what follows is
  `js::ends`' answer, the one MIR's tidy pass asks too; preparation had
  its own, which knew only `return` and `throw` (the second architecture
  review).
