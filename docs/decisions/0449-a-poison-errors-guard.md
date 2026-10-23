# 0449. A `PoisonError` gives its guard, and a lock's `Ok` is its guard however it's unwrapped

Status: Accepted. Extends [0144](0144-locks.md) and
[0327](0327-cell-deque-char-methods.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a lock is never
poisoned in rust-js (ADR 0327), so its `Err` is never made, and what
handles one gives what Rust's gives.

## Context

Code that shares a lock takes its guard whether or not another thread
panicked holding it: `m.lock().unwrap_or_else(|e| e.into_inner())`, or
`unwrap_or_else(PoisonError::into_inner)`, as uuid's clock sequence does.
`PoisonError::into_inner` was refused, and with it uuid.

A lock's `Ok` is made where it's locked, and `unwrap()` of an `Ok` just
made was already its value (ADR 0144), `$lock(m, true)`; `unwrap_or_else`
of one tested it, `r.TAG === "Ok" ? r._0 : ..`, for an `Err` that can't be.

## Decision

**A `PoisonError`'s `into_inner()` and `get_ref()` are the guard it holds,
its field, `e.data`. And `unwrap_or_else(f)`, `unwrap_or_default()` and
`unwrap_or(d)` of an `Ok` just made are what it holds, as `unwrap()`'s is,
`f` and `d` not made where nothing they do can be seen.**

```rust
let mut n = m.lock().unwrap_or_else(|e| e.into_inner());
*n += 1;
```

```js
const n = $lock(m, true);
n.value = (n.value + 1) >>> 0;
```

1. **`into_inner()` and `get_ref()`** read the `PoisonError`'s field, as
   its type's fields are read anywhere: `e.data`. It moves its guard out,
   so a guard with a destructor is given back, not dropped. `get_mut()`
   isn't yet: a `&mut` to a guard of a number is a box (ADR 0074).
2. **An `Ok` just made**, `{ TAG: "Ok", _0: x }`, unwrapped in any of the
   ways a `Result` is: `x`. One way for each, as `unwrap()`'s, so a lock
   reads the same however it's taken.
3. **A comparison of two strings written out is its answer**, as one of
   two numbers is: `"Ok" === "Ok"`, a tag's test of a variant made here,
   is `true`, so `unwrap_or(loud())` of `Ok(9)` is `loud(); const nine =
   9;`, its fallback made first as Rust makes it. A program's own, `"a" ==
   "b"`, is its answer too.
4. **One function answers it**, `result::unwrapped_made`, which each way
   of unwrapping asks.
5. **A closure read in place, called where it's made**, `((e) =>
   e.data)(r._0)`, of what reads the same: its value, `r._0.data`, as
   `calls::apply` makes of a closure given in place. Tidy asks it once a
   closure's `const` is read where it's called.

## Why

- **It's how shared Rust takes a lock**: poisoning is a server's concern,
  and code that runs on both sides handles it as uuid does.
- **It reads as a person writes it**: `$lock(m, true)`, not a test of a
  `Result` that is always `Ok`.
- **It's tested**: a corpus case takes a lock each way, holds a lock's
  `Result` and reads its `Err` side, and unwraps an `Ok` just made each
  way, against native Rust; a mutation of each part is caught.

## Alternatives

- **A lock's `lock()` as its guard, not a `Result`**: no `Result` to
  unwrap, but `match m.lock()` and `?` read one, and its type is Rust's.

## Consequences

- uuid gets past its clock sequence's lock, to `Deref` of a generic
  `MutexGuard<C>`, the next gap.

## Amended (2026-10-24)

A closure called where it's made is put in place in one way: tidy's
`applied arrows`, of what reads the same. `calls::apply_in`, which put a
one-statement closure in place where `LocalKey::with` was lowered, was a
second way, and made no JS of its own: with it gone, the corpus's JS is
the same, and its mutations survived. It's gone, and `with` calls its
closure, which that pass puts in place. `test/lowering-cells.test.ts`'s
"with of a one-statement closure is its statement" holds it.

