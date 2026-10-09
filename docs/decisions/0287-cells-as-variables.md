# 0287. A cell only its function reads and sets is a `let`

Status: Accepted. Extends [0023](0023-strings-references-shared-state.md)
and [0270](0270-module-variables.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `Cell` is `{ value }` (ADR 0023), so everyone sharing it sees a change.
A local one usually shares nothing JS doesn't share already: JS closures
capture variables, not values. react.dev's Preview keeps `let timeout`,
which its bundler listener sets and its cleanup clears. Rust needs an
`Rc<Cell<_>>` for that, and a clone of it for each closure, which was:

```js
const timeout = {};
const started = timeout;
listen((message) => { started.value = setTimeout(..); });
return () => { clearTimeout(timeout.value); };
```

ADR 0270 made a thread-local only read and set its module's variable.

## Decision

**A local `Cell`, or an `Rc` of one, that only its function and its
closures read and set, by `get` and `set`, is its function's `let`, and a
clone of it, `let started = timeout.clone()`, is that variable too.**

```js
let timeout;
listen((message) => { timeout = setTimeout(..); });
return () => { clearTimeout(timeout); };
```

- `let n = Cell::new(x)` and `let n = Rc::new(Cell::new(x))` are `let n
  = x`; `n.get()` is `n`, and `n.set(v)` is `n = v`.
- A closure captures it, `move` or not, as JS does: it isn't copied.
- One used any other way is a cell, and so is each of its clones: given
  or returned whole, `replace`d, `take`n, compared, or cloned anywhere
  but a `let` of its own.
- It changes, so what reads it is read where Rust reads it: JSX takes
  `{n.get()}` before a later `n.set(5)` (ADR 0218).
- `let x = undefined;` is `let x;`, as a person writes it.

## Why

- **It's the JS a person writes**, and the same program: a variable
  every closure captures is one place, as the cell was.
- **Conservative**: anything but `get`, `set` and a clone is a cell, as
  before.

## Consequences

- The corpus, examples and the playground lose their `{ value }`s of
  counters and flags: `let calls = 0; calls = (calls + 1) | 0;`.
