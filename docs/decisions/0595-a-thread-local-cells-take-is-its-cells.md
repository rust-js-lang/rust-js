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
taskTimeoutId = undefined;`, as a `Cell`'s are (ADR 0585). (Amended: a `RefCell`
key's are its cell's too, below.)

## What holds it

- `test/lowering.test.ts`, "a thread-local Cell's take and replace are its
  variable's read and write".
- Mutation: `take` refused.

## Amended: a thread-local `RefCell`'s `take` and `replace` are its cell's too

React's reconciler takes its queue of concurrent updates whole,
`QUEUE.take()`, which was refused; the port wrote
`with_borrow_mut(std::mem::take)` in its place, which isn't what one
writes.

**A `LocalKey<RefCell<T>>`'s `take` and `replace` are its `RefCell`'s**,
as `RefCell::take` and `RefCell::replace` are (ADRs 0585, 0863): a
mutable borrow for a moment, checked where a borrow of it may be held,
`$take($borrowMut(ports), [])`, which throws as Rust panics, and else
the cell written in place, `$take(queue, [])` of an array, `const old =
count; count = 10;` of a value. Each is a write of the thread-local, as
its `set` is, so one taken or replaced stays its module's variable (ADR
0270), and its `with_borrow_mut` its closure's body; one exported and
written outside its module is refused as a `set` is.
`recognition::cell_of_key` answers what a key's own `take` is of; the
lowering of `RefCell`'s `take` and `replace` asks it.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): JS's own read and
write, or the `RefCell`'s borrow, as Rust's.

- `test/corpus/thread_local_refcell_take.rs`: each of a thread-local
  `Vec`'s, number's, `String`'s, `Option<Rc<_>>`'s and `Cell`'s `take`
  and `replace`, against native Rust; `thread_local_refcell_take_borrowed.rs`,
  one taken within its `with_borrow`, panicking as Rust's does.
- Mutations: `thread-local-refcell-take-refused`, `thread-local-take-boxes-it`,
  `thread-local-take-of-key`.
