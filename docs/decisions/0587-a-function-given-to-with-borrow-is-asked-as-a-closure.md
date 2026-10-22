# 0587. A function given to `with_borrow` is asked what it borrows

Status: Accepted. Extends [0270](0270-module-variables.md)'s plain
thread-locals and [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A thread-local is
its variable only where nothing it runs while borrowed can ask; one that
can stays a cell, and throws where Rust panics.

## Context

React's scheduler reads its current task as a clone:

```rust
while let Some(current_task) = CURRENT_TASK.with_borrow(Clone::clone) {
```

A thread-local is its module's variable where each `with_borrow` runs a
closure that borrows nothing that may be it (ADRs 0270, 0580). A function
given, `Clone::clone`, wasn't a closure, so it was taken to ask, and the
task stayed a cell, `const currentTask = {}`, read by
`$withBorrow(currentTask, (arg0) => arg0)`. Upstream's is `var
currentTask = null`.

## Decision

**What `with_borrow` is given to run, a closure or a function, is asked
what it may borrow**: a closure by its body's summary, a function as a
call of it is (`CellBorrows::call_asks`): std's `clone` borrows nothing;
a function of the crate's what its body may; anything else, anything.

```js
let currentTask;
..
const currentTask$1 = currentTask;
```

## Why

- **One question, asked of what runs**: whether running it may ask of
  the key held, whether it's written `|t| t.clone()` or `Clone::clone`,
  which clippy prefers.

## What holds it

- `test/lowering.test.ts`, "a thread-local given a function to run is its
  variable where it borrows nothing": `with_borrow(Clone::clone)` plain,
  and a function that borrows its own key mutably, still a cell, whose
  borrow throws.
- Mutations: a function given never asked, always quiet.
