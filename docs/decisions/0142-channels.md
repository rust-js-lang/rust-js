# 0142. A channel on one thread is a queue its ends share

Status: Accepted. Extends [0098](0098-destructors.md). Amended:
`thread::yield_now()` is nothing (below).

Case: C, B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`std::sync::mpsc::channel()` is how Rust code hands values from producers
to a consumer. On one thread, which is all JS has, it's a queue, but not
only that: whether it's still connected is part of what it does.

```rust
let (tx, rx) = mpsc::channel();
produce(tx);                         // sends, then drops `tx`
while let Ok(job) = rx.recv() { .. } // ends once the queue is empty and
                                     // every sender is gone
```

`recv()` of an empty channel is an `Err` once every sender is dropped, and
`send` is an `Err` once the receiver is. A queue alone can't tell either.

## Decision

**A channel is a JS object its two ends share,** `{ queue, senders,
receiving }`, and each end is that object. The ends are values with
destructors (ADR 0098), which keep the count:

| Rust | JS |
|---|---|
| `let (tx, rx) = mpsc::channel();` | `$channel()`, `[channel, channel]` |
| `tx.send(item)` | `$send(tx, item)`: `Ok`, or `Err(SendError(item))` with no receiver |
| `rx.recv()` | `$recv(rx)`: the first item, or `Err(RecvError)` with no senders |
| `rx.try_recv()` | `$tryRecv(rx)`: or `Err(Empty)` or `Err(Disconnected)` |
| `tx.clone()` | `$cloneSender(tx)`: one sender more |
| a sender dropped | `$dropSender(tx)`: one sender fewer |
| the receiver dropped | `$dropReceiver(rx)`: none receives |

- **`recv()` of an empty channel whose senders are there panics,** with a
  message that says so. Rust's waits for another thread to send; on one
  thread none can, and it would wait forever.
- **`{:?}` of its errors is std's:** `RecvError`, `Empty`,
  `Disconnected` and `SendError { .. }`.
- **A channel of a value with a destructor is an error:** the queue would
  drop what's still in it, its own way.

## Why

- **The program's own loop ends as Rust's does.** A loop over `recv()`
  stops when the last sender is dropped, which is what a destructor is for.
- **One object for both ends** is all the state there is; a sender's
  identity doesn't matter, only how many there are.

## Alternatives

- **A queue, and no ends.** `recv()` couldn't tell an empty channel from a
  closed one: the loop above would never end, or end too soon.
- **Waiting for another send.** JS can't block, and nothing else can send
  while it waits.

## Consequences

- `sync_channel`, `recv_timeout`, `iter()` and `try_iter()` are still errors.
- `std::thread::spawn` is an error, so both ends are always on the one thread.
- **`thread::yield_now()` is nothing** (amended, 2026-10-15): the one
  thread there is yields to none, which no program can see (case A of ADR
  0262). It's std's `Nothing`, as a `Vec`'s `reserve` is (ADR 0315); the
  corpus's `thread_yield.rs`, and rustc's `threads-sendsync/yield2.rs`,
  which was refused, pass.

## Amendment: a channel of values with a destructor

A channel of a value with a destructor was an error, as the queue would
drop what's still in it. It drops it as Rust's does: **what's still queued
is dropped with the receiver, in the order it was sent,**
`$dropReceiver(rx, drop)`, the item's drop function given where the
receiver's dropped. What's received is the receiver's caller's to drop, as
any value is; a send after the receiver's gone gives its value back, in
its `Err`, to the caller's; and a sender's drop drops nothing queued, which
the receiver may still take. So `send` of one is no error, the queue its
owner. A channel of senders, rustc's `task-comm-16` and
`send-type-inference`, which pass: a queued sender dropped with the
receiver is one sender fewer of its own channel. The `channel_of_droppers`
corpus case runs each, with native Rust.
