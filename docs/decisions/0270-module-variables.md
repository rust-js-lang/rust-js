# 0270. A thread-local only read and set is its module's variable

Status: Accepted. Amends [0037](0037-thread-locals.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page caches the error codes it fetched in a variable of
its module:

```tsx
let cachedErrorCodes: Record<string, string> | null = null;
```

A Rust module keeps one in a `thread_local!`'s `Cell`, which rust-js made
a `{ value }` (ADR 0037), so anyone sharing the cell sees it change:

```js
const cachedErrorCodes = { value: undefined };
cachedErrorCodes.value = codes;
```

## Decision

**A thread-local only read and set, by `get`, `set`, `with_borrow` and
`with_borrow_mut`, in its own module, and not public, is its module's
variable of what its cell holds**: a `let` if it's `set`, a `const`
otherwise.

```rust
thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
COUNT.set(COUNT.get() + 1);
LOG.with_borrow_mut(|log| log.push(line.to_string()));
```

```js
let COUNT = 0;
const LOG = [];
COUNT = (COUNT + 1) >>> 0;
```

- **One whose cell is handed out, `with(|cell| ..)`**, keeps its
  `{ value }`: what holds the cell must see it change.
- **One another module reads**, or that's public, keeps it: an importer
  can't set another module's `let`.

## Why

- **No program can tell**: nothing but its module's reads and sets reaches
  the cell, and each reads or sets the variable instead.
- **It's the JS a person writes.**
- **It's tested**: a compiler test sets a `Cell`'s and changes a
  `RefCell`'s `Vec`, beside a public one and one another module reads;
  mutations box them all, unbox the other module's, and make a set one a
  `const`. The thread-locals example and its snapshot say each.

## Since

- **A `const { .. }` thread-local is one too**, by its own name: std puts
  its value in a `const` of its block, `__RUST_STD_INTERNAL_INIT`, which was
  written by that name while the thread-local was read by its own, as
  react.dev's errors page's `cachedErrorCodes` found. And one that starts
  `None`, set later, is `let SEEN;`, as JS starts a variable `undefined`.
