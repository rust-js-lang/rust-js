# 0595. A thread-local `Cell`'s `take` and `replace` are its cell's

Status: Accepted. Extends [0037](0037-thread-locals.md)'s thread-locals and
[0585](0585-a-refcells-replace-holds-it-for-a-moment.md)'s `take` and
`replace`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A.

## Context

React's scheduler clears the timeout it set, taking its id:

```rust
js::clear_timeout(TASK_TIMEOUT_ID.take());
```

A `LocalKey<Cell<T>>`'s `get` and `set` were its cell's (ADR 0037); its
`take` and `replace`, std's too, were refused.

## Decision

**A thread-local `Cell`'s `take` and `replace` are its cell's**: the old
value read and the new one written, `const old = taskTimeoutId;
taskTimeoutId = undefined;`, as a `Cell`'s are (ADR 0585). A `RefCell`
key's, which borrow it, stay refused.

## What holds it

- `test/lowering.test.ts`, "a thread-local Cell's take and replace are its
  variable's read and write".
- Mutation: `take` refused.
