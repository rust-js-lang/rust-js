# 0237. An `if` whose branch leaves has no `else`

Status: Accepted.

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
- **Its locals stay apart**, now in the block the `if` is in: every local
  of a function has a name of its own (ADR 0010).
- **It's how the JS is written, not what rust-js lowers**: the statements
  are the same, written as oxc's.

## Why

- **It's the JS a person writes.**
- **It runs the same**: after a branch that leaves, nothing that follows
  runs, and nothing else did; inside a `try`, a `return` still runs its
  `finally`.
- **It's tested**: a compiler test writes and runs an `else if` chain that
  returns, a branch that leaves by an inner `if`, and one that doesn't;
  the corpus runs every changed program beside native Rust.
