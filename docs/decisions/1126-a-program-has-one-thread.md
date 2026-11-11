# 1126. A program has one thread

Status: Accepted, the user's decision (#1176, 2026-11-11). Amends
[0142](0142-channels.md), a channel on one thread, and
[0144](0144-locks.md), locks.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where one thread runs
as Rust's does on one, the same results; a compile error where a program
starts another thread; a refusal as it runs where Rust's would wait
forever.

## Context

JS has one thread per realm; a Web Worker shares nothing but a
`SharedArrayBuffer`. Most crates use `Arc`, `Mutex` and atomics without
starting a thread, and the sync crates' building blocks, crossbeam-utils',
fastrand's, parking_lot_core's, tracing-core's, scopeguard's and log's,
ask what this thread is doing: `LocalKey::try_with`, `thread::panicking`
and `hint::spin_loop` (#1285), refused before, about 250 of the Crater's
tests.

## Decision

**A program has one thread, and what it has of threads works as it does
on one, which is exactly Rust's there. Starting another is refused at
compile time.**

- **`Arc`, `Mutex`, `RwLock`, atomics, `OnceLock` and the other sync
  primitives** work as on one thread: a lock is free unless its thread
  holds it (ADR 0144), an atomic is its value.
- **A thread-local's `try_with(f)`** is `Ok(f(key))`. Its `Err`, after the
  key's destructor, never comes: a thread-local is a `static` on wasm32,
  whose destructor never runs.
- **`thread::panicking()`** is whether this thread unwinds: the depth of
  the cleanups a panic runs, which every unwinding drop runs in
  (`$cleanup`), so a drop a panic runs sees `true` and one after its
  `catch_unwind` `false`.
- **`hint::spin_loop()`** does nothing, as `thread::yield_now()` doesn't:
  no other thread is waited for.
- **A channel's `iter()`** gives each item `recv` gives and ends once every
  sender is gone; `try_iter()` gives what's queued. Where Rust's `recv`,
  or its `iter`, would wait forever on an empty queue whose sender is
  still there, it's refused as the program runs, `$unsupported`, which no
  `catch_unwind` takes for a panic, as Rust's never returns. It was a panic
  before, which a `catch_unwind` took.
- **`thread::spawn`, a `Builder`'s and a scope's `spawn`** are refused at
  compile time, naming this ADR. Workers may come later, where a program
  needs them.

## Consequences

- crossbeam-utils', fastrand's, parking_lot_core's, tracing-core's,
  scopeguard's and log's building blocks compile.
- A lock is still never poisoned (ADR 0144). With `thread::panicking()`
  answered, a guard's drop could poison it as Rust's does; that's left to
  its own decision.
- Corpus case one_thread_sync covers each against native Rust in both
  builds; `test/lowering.test.ts` covers a channel's iteration that would
  wait forever.
