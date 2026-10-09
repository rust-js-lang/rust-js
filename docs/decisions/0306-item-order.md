# 0306. A module's items are where its Rust has them

Status: Accepted. Amends [0037](0037-thread-locals.md) and
[0267](0267-on-load.md): where a module's `const`s and statements are.

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what's made when
it's loaded, N for where the rest is.

## Context

rust-js wrote a module's items by kind: its types' methods, its `const`s,
what it runs when it's loaded, then its functions. react.dev's runESLint
has its function first, then its `const`s, a statement between them:

```ts
const getCodeMirrorPosition = (doc, {line, column}) => { .. };
const linter = new Linter();
const reactRules = require('eslint-plugin-react-hooks').rules;
linter.defineRules({ .. });
const options = { .. };
export const runESLint = (doc) => { .. };
```

And by kind, its `const`s were as written, so one reading a later one,
which Rust makes when it's first read, threw where JS made it:

```js
const AFTER = (LATER + 1) >>> 0; // ReferenceError: LATER isn't made yet
const LATER = 10;
```

## Decision

**A module's items are where its Rust has them; what's made when it's
loaded, a `const`'s value or an `on_load!`'s statements, comes after the
`const`s and types it reads, through the functions it calls too.**

```rust
thread_local! {
    static CALLED: u32 = read_later() + 1;
}
thread_local! {
    static LATER: u32 = 10;
}
fn read_later() -> u32 {
    LATER.with(|later| *later)
}
```

```js
const LATER = 10;
const CALLED = (read_later() + 1) >>> 0;

function read_later() {
  return LATER;
}
```

- An item written in another, a `const` in a function's body, is just
  before it, and a function's place is its whole item, body and all.
- What it reads is every name in it, closures' too, and in the functions
  and types' methods it names: a closure may run when it's made.
- Of a cycle, the first written comes first.

## Why

- **It's the JavaScript a person writes**: react.dev's order, as its
  port's Rust has it.
- **It's the same program**: what Rust makes when it's first read is made
  before JS reads it. A function is a declaration, hoisted, so it's where
  it's written.
- **It's tested**: a module test runs a thread-local reading a later one,
  directly and through a function, and an `on_load!` before what it sets;
  mutations order items by kind, drop what's read, what a called function
  reads, and put a function's `const` after it.

## Costs

- **A closure that's only kept may move a `const`**: reading one in a
  closure that runs later doesn't need it made first, but the closure
  can't tell, so it's after it all the same.
- **Thread-locals that read each other when made** throw where JS makes
  the first, as Rust's recursive initialization fails.
