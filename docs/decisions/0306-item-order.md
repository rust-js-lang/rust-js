# 0306. A module's items are where its Rust has them

Status: Accepted. Amends [0037](0037-thread-locals.md) and
[0267](0267-on-load.md): where a module's `const`s and statements are.
Amended below: a function an initializer only holds isn't run.

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

## Amendment: a function an initializer only holds isn't run

React's clz32.js picks `Math.clz32` or its fallback, then writes the
`const`s the fallback reads:

```js
export const clz32 = Math.clz32 ? Math.clz32 : clz32Fallback;

// Count leading zeros.
const log = Math.log;
const LN2 = Math.LN2;
function clz32Fallback(x) { .. }
```

Its port, in the same order, a thread-local first, printed `const { log,
LN2 } = Math;` and its comment above `export const clz32`: what an
initializer reads was every name in it and in the functions it names, and
`clz32Fallback` reads `log` and `LN2`. But naming a function doesn't run
it: the initializer only holds it, as its value.

**A name an initializer's value only holds, the value itself, a
conditional's branch, an `||`'s, `&&`'s or `??`'s operand, an array's
item or an object's field (`Expr::visit_held_vars`), is made before the
initializer, but what its function reads isn't: the initializer is where
its Rust has it.** What reads that `const` when it's loaded may call what
it holds, so it's after what that function reads, through any `const`
holding it in turn:

```rust
thread_local! {
    static PICK: fn(u32) -> u32 = scaled;
    static PICKED: [fn(u32) -> u32; 1] = [PICK.with(|pick| *pick)];
    static USED: u32 = PICKED.with(|picked| picked[0](2));
    static SCALE: u32 = 3;
}
```
```js
const pick = scaled;
const picked = [pick];
const SCALE = 3;
const USED = picked[0](2);
```

- **Anything else runs**: a call's callee or argument, an operand of any
  other operator, a member's object, a closure (Costs, above), a spread,
  an object's getter or method. Each is read as before, through the
  functions it names.
- **An item moves only where what it reads isn't made yet**: everything
  else is where its Rust is, as before.

Case N ([0262](0262-when-rust-and-js-disagree.md)): where an item is,
when what it reads is made before it's read either way.

Test: `modules`'s "a function an initializer only holds doesn't move what
it reads before it": clz32's port, its JS as upstream's lines, run; and
`USED` after `SCALE`, run where JS would throw if not. Mutations
`load-held-run`, `load-holds-not-carried` (`js/stmt_walk.ts`),
`held-branch-run`, `held-array-item-run` (`js/expr_walk.ts`).
