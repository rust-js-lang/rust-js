# 0328. A `RefCell` counts its borrows, as std's does

Status: Accepted. Amends [0025](0025-vec-loops-refcell-mut.md),
[0144](0144-locks.md), [0270](0270-module-variables.md) and
[0327](0327-cell-deque-char-methods.md); extends
[0098](0098-destructors.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), and D, said at run
time, for a lock locked while its thread holds it.

## Context

ADR 0025 made a guard what it guards, so a `borrow_mut()` while a
`borrow()` lived ran on in JS where Rust panics: a silent drift. ADR 0327
refused `try_borrow` and `try_borrow_mut`, which ask of a borrow's state,
and a lock locked again by the thread that holds it, which deadlocks in
Rust, ran on too.

## Decision

**A `RefCell`, a `Mutex` and an `RwLock` count the guards held, `borrows`;
a guard is its cell, dropped where Rust drops it (ADR 0098), and its
deref is the cell's `value`.**

```rust
let reading = cell.borrow();
cell.borrow_mut().push(2);
```

```js
const reading = $borrow(cell, true);
try {
  $borrowMut(cell).value.push(2); // throws "RefCell already borrowed"
} finally {
  $unborrow(reading);
}
```

| Rust | JS |
|---|---|
| `c.borrow()`, `c.borrow_mut()`, held | `$borrow(c, true)`, `$borrowMut(c, true)`, `$unborrow(g)` where it's dropped |
| one held only while nothing can ask | `$borrow(c).value.length`, `$borrowMut(c).value.push(x)`: checked, not counted |
| `*g`, `*g += 1` | `g.value`, `g.value += 1` |
| `try_borrow()`, `try_borrow_mut()` | `$tryBorrow(c)`, `$tryBorrow(c, true)`: `Ok` of a held guard, or `Err` of its message |
| `replace(x)`, `take()`, `swap(&o)`, `replace_with(f)` | `$cellReplace($borrowMut(c), x)`, `$refCellSwap(c, o)`, `$replaceWith(c, f)` |
| `clone()`, `==`, `{:?}` | of `$borrow(c).value`, or in `$withBorrow(c, f)` where `T`'s may ask; `<borrowed>` while mutably borrowed |
| `m.lock()`, `l.read()`, `l.write()` | `Ok` of `$lock(m)`, `$lockRead(l)`, `$lock(l)`, held or checked as a borrow is |
| a thread-local's `with_borrow(f)`, `with_borrow_mut(f)` | `$withBorrow(KEY, f)`, `$withBorrowMut(KEY, f)`, or `f(KEY)` of a plain one |

- **Counted only where something could ask.** A guard a statement makes
  and drops, `c.borrow_mut().push(x)`, is checked and not counted where
  nothing in its temporary scope may ask whether a cell is borrowed: no
  call of the crate's, a closure's or a `dyn`'s; none of std's that
  borrows a cell, or whose bounds the crate's impls meet, followed through
  std's impls, `Vec<T>: Clone` to `T: Clone`; and nothing dropped but
  guards. A shared borrow beside a shared one asks nothing it changes.
  Counting it there would change nothing: a panic ends the program
  (ADR 0035).
- **A held guard is a value with a destructor** (ADR 0098): a scope that
  holds one is a `try`, `drop(g)` releases it, and generic code is given
  its drop.
- **`borrows` isn't enumerable**: `==`, a spread, a clone and JSON see a
  cell's `value` alone. `==` of a mutably borrowed `RefCell` panics, as its
  borrow does.
- **A thread-local stays plain (ADR 0270)** where each `with_borrow`
  closure can't ask; otherwise it's its cell, and `f` runs while it's
  borrowed.
- **A lock locked while its thread holds it throws** `rust-js does not
  support locking a lock its thread holds, which deadlocks in Rust`: ADR
  0262's D, at run time. A read beside another read isn't one.
- **Still refused**: `try_lock`, `try_read` and `try_write`, whose
  `TryLockError` rust-js has no form for yet; `Ref::map` and a guard's other
  own functions; and `{:?}` of a lock.

## Why

- **It's the same program**: each borrow panics where std's does, and
  `try_borrow` answers as std's does.
- **It's the JS a person writes where it can be**: most borrows are
  momentary, one check each, and only a guard that lives while code runs
  is counted and released.
- **It's tested**: corpus cases run against native Rust: a `borrow_mut()`
  beside a `borrow()`, a `borrow()` beside a statement's `borrow_mut()`;
  `replace`, `clone`, `==` and `swap` with itself while borrowed; shared
  borrows, `try_borrow*`, `drop`, writes through a fresh guard; a
  thread-local borrowed again in its closure; and code that asks from a
  temporary's `Drop`, an item's `Clone` behind `Vec` and `Option`, a
  closure, and a cell's own `clone`, `==` and `{:?}` of a value that holds
  a `Weak` to it. A lock locked again, and read under a write, is
  rust-js's error. Mutations leave a borrow unchecked or uncounted, let a
  shared borrow block another, keep a dropped guard's count, enumerate the
  count, take a call, a drop, a bound or a closure for quiet, count every
  borrow, skip `std`'s impls' bounds, keep a thread-local plain under a
  closure that asks, copy a guarded number, and show a borrowed cell.
- `docs/std-coverage.txt`: `RefCell` 11 of 13.
