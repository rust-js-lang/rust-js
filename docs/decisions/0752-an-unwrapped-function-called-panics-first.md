# 0752. An `Option`'s function unwrapped and called panics before the call

Status: Accepted. No change to the compiler: `$unwrap` is kept where
the value it unwraps is called, as it is where it's read. Amended by
[0873](0873-a-function-handed-to-js-carries-no-drop.md) (2026-10-25): where
its panic drops a task no scope's `finally` does, it's its test, `if
(enqueueTaskImpl == null) { $dispose(task); throw new Error(..); }
enqueueTaskImpl(task);`, which panics before the call too.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, Rust's: a `None`
called would change what the program does.

## Context

React's `enqueueTask` (shared/enqueueTask.js) ends by calling the
implementation it found, which it set on each way the `if` before goes:

```js
return enqueueTaskImpl(task);
```

The port held it in a `Cell<Option<fn(Box<dyn Fn()>)>>` and wrote its
last line `if let Some(enqueue_task_impl) = ENQUEUE_TASK_IMPL.get() {
enqueue_task_impl(task); }`, which rust-js writes `if (enqueueTaskImpl) {
enqueueTaskImpl(task); }`. That's a different program from upstream's: of
a `None`, upstream throws and the port does nothing. The Rust upstream's
is `ENQUEUE_TASK_IMPL.get().unwrap()(task)`, which a port must write
(AGENTS.md, "A port is faithful"). What should rust-js write of it?

## Decision

**`f.unwrap()(x)` is `$unwrap(f)(x)`**, as rust-js writes it already: the
`unwrap` panics of a `None`, before `x` is read, with Rust's message,
``called `Option::unwrap()` on a `None` value``. The port's last line is
`ENQUEUE_TASK_IMPL.get().unwrap()(task)`, and its JS
`$unwrap(enqueueTaskImpl)(task);`.

## Why

- **The program can tell a bare call apart**: `enqueueTaskImpl(task)` of
  `undefined` evaluates `task`, and anything an argument does, before it
  throws, and throws a `TypeError`, not the panic a `catch_unwind` or a
  panic hook sees. rust-js can't prove the cell holds a function there:
  each branch before sets it to what a JS call gave, which may be
  `undefined` (`timers.setImmediate` of a host without one).
- **It's one call shorter than the port's `if`, and the same length as
  upstream's**, but for `$unwrap`.

## Alternatives

- **A bare call, under case B**: shorter, but a difference rust-js would
  have to list for every `unwrap()` called, and one that reorders an
  argument's effects; not rare, as every `Option` of a callback is one.
- **The port's `if let`**: a narrower program than upstream's, which a
  port isn't (AGENTS.md).

## Consequences

- `test/port-performance-track.test.ts` holds the port's last line.
