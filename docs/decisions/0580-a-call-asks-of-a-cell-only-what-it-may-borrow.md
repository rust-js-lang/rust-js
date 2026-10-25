# 0580. A call asks of a cell only what it may borrow

Status: Accepted. Extends [0328](0328-refcell-borrow-counts.md), whose
"nothing can ask" counted any call of the crate's as one that may, and
[0270](0270-module-variables.md)'s plain thread-locals.
Amended by [0863](0863-a-refcell-of-an-object-is-the-object.md): std's code
a call runs runs the crate's impls its bounds name. Extended by
[0897](0897-an-element-a-guard-reaches-is-its-place.md): an intrinsic runs
only what it's given, so `vec![..]` runs nothing. Extended by
[0898](0898-a-binding-that-runs-only-what-its-given.md): a binding that runs
only what it's given, given only numbers and text, borrows nothing.
Extended by [0899](0899-a-fn-pointer-runs-what-the-crate-made-it-of.md):
a fn pointer's call runs what the crate made one of its type of, not
anything, where no value of its type comes from outside.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A borrow left
uncounted is one no other borrow of its cell can meet. A JS getter that
calls back into Rust and borrows a cell held, rare and harmless, isn't
seen (below).

## Context

React's scheduler moves each timer that's due from one heap to the
other:

```rust
TIMER_QUEUE.with_borrow_mut(|timer_queue| {
    while let Some(timer) = peek(timer_queue) {
        ..
        let timer = pop(timer_queue).expect("peeked");
        TASK_QUEUE.with_borrow_mut(|task_queue| push(task_queue, timer));
    }
});
```

A thread-local is its module's variable where each `with_borrow` closure
can't ask whether it's borrowed (ADRs 0270, 0328). ADR 0328 took a call of
any function of the crate's as one that may, so `peek`, `pop` and `push`,
which borrow no cell, made each queue a `{ value }` cell and each closure
`$withBorrowMut(timerQueue, ..)`, its borrow counted. Upstream's queues
are `var taskQueue = []`.

## Decision

**What each body of the crate may borrow is known, of its cells**
(`analysis/cell_borrows.rs`): a thread-local's by its key, a `RefCell`'s,
a `Mutex`'s or an `RwLock`'s by its type, each borrow shared or mutable.
A body borrows what it borrows itself, what each function of the crate's
and each closure it makes may, of the call's types, and anything where it
calls what rust-js can't see, a `dyn`'s method, a fn pointer, or a
binding that runs JS, or drops a value whose destructor runs (ADR 0320).
A type parameter's drop is its caller's: a summary keeps `T`'s, and a call
says whether `T` of it destructs.

**A borrow asks of one held only where it may be of its cell, and one of
them is mutable**: a thread-local's of its key, or of its cell's type;
two types alike. So:

- a `with_borrow` closure is quiet of its key where it borrows nothing
  that may be it: `TIMER_QUEUE`'s calls `TASK_QUEUE`'s and the heap's;
- a guard is momentary where nothing in its extent may borrow its cell
  (ADR 0328's `Quiet`, of the one question `call_asks`);
- a binding that reads a property, `node.sortIndex`, `in`, `instanceof`
  or `!!`, runs nothing, as one that makes an element or an object
  already did.

`may_ask_borrows` and `asks_no_borrows`, which answered "may this ask" of
any cell, are gone: `CellBorrows` answers it, of a cell.

```js
const taskQueue = [];
const timerQueue = [];
```

## Why

- **The question is of a cell**: a borrow of `TASK_QUEUE` can't fail
  for `TIMER_QUEUE` being held, and a function that borrows nothing can't
  fail any.
- **One owner**: what a call may borrow is `CellBorrows`', asked by the
  thread-locals and the momentary guards alike.

## Alternatives

- **Inline the crate's functions to see what they borrow**: what a
  summary says, per call. Rejected.
- **Trust every function of the crate's not to borrow**: a closure that
  borrows its own thread-local again would no longer throw where Rust
  panics. Rejected.

## Consequences

- No corpus case's JS changes; React's scheduler's queues are its arrays.
- A JS getter that calls back into Rust, borrowing a cell held where a
  property is read, isn't seen: its borrow passes where Rust's panics.
  No binding rust-js has does it.

## What holds it

- `test/lowering.test.ts`, "a thread-local whose closures call only what
  can't borrow it is its variable": two queues and the functions moving
  between them plain, and a closure that borrows its own key through a
  function still a cell, whose second borrow throws.
- `test/lowering.test.ts`, "a field's RefCell only borrowed for a moment
  is its value", and the corpus's `thread_local_borrow_inside`.
- Mutations: a key never its own, a function of the crate's borrowing
  nothing or anything, a body asked of nothing, an element's binding taken
  to run code, a call beside a guard taken as asking nothing.
