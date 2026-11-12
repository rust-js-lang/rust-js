# 0144. A lock on one thread is a `RefCell`, and an `Arc` is an `Rc`

Status: Accepted. Extends [0023](0023-strings-references-shared-state.md)
and [0025](0025-vec-loops-refcell-mut.md). Amended: a lock a panic poisons
is poisoned (#1438).

Case: A, B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Shared state in Rust that may cross threads is `Arc<Mutex<T>>` or
`Arc<RwLock<T>>`, and it's how much Rust is written even where it never
does:

```rust
let stats = Arc::new(Mutex::new(Stats { hits: 0 }));
let other = Arc::clone(&stats);
other.lock().unwrap().hits += 1;
```

Both were errors: `Arc` wasn't an `Rc`, and `lock()` wasn't anything.

JS runs a module on one thread, and `std::thread::spawn` is an error
(ADR 0142), so no other thread can hold a lock: a lock is a `RefCell` that
nothing contests.

## Decision

**A `Mutex` or an `RwLock` is `{ value }`, as a `RefCell` is, and an `Arc`
is the value it points at, as an `Rc` is:**

| Rust | JS |
|---|---|
| `Arc::new(x)`, `Arc::clone(&a)` | `x`, `a` |
| `Mutex::new(x)`, `RwLock::new(x)` | `{ value: x }` |
| `m.lock()`, `l.read()`, `l.write()` | `{ TAG: "Ok", _0: m.value }` |
| `m.lock().unwrap()`, `.expect(..)` | `m.value` |
| `*m.lock().unwrap() += 1` | `m.value = (m.value + 1) >>> 0` |
| `m.into_inner()`, `m.get_mut()` | `Ok` of `m.value` |

- **A lock is always `Ok`:** nothing else can hold it, and a guard isn't
  poisoned without another thread's panic. `unwrap()` of that `Ok` is its
  value, as for any `Ok(x)` just made.
- **A guard is what it guards,** `MutexGuard`, `RwLockReadGuard` and
  `RwLockWriteGuard` as `Ref` and `RefMut` are (ADR 0025): one held in a
  variable is the object.
- **A guard of a number, a string or the like held in a variable names the
  cell's `value`,** as a `&mut` held in one names its place (ADR 0099):
  `let mut n = m.lock().unwrap(); *n += 1;` is `m.value = (m.value + 1) | 0`,
  and so is a `RefCell`'s `borrow_mut()`. While the guard lives nothing
  else can use the cell, so a write through it is a write to `m.value`. Its
  place is fixed where it's locked: `slots[i].lock()` keeps the `i` of
  that moment. One of a cell that isn't a place, a call's result, is still
  an error.
- **An `Arc` is wherever an `Rc` is:** shown, compared, cloned, defaulted
  and written to JSON as what it holds.
- **Still errors:** `try_lock`, `is_poisoned`, `Arc::strong_count`, `{:?}`
  of a lock, which shows whether it's locked, and a lock of a value with a
  destructor.

(Amended by ADR 0328: a lock counts its guards as a `RefCell` does, a
guard is its cell, and one locked again while its thread holds it is
rust-js's error, where Rust deadlocks.)

## Why

- **It's exact on one thread.** A lock only changes what a program does
  when another thread holds it, and none can.
- **It's the JS a person writes:** a shared object, and a field read or
  written, with no lock at all.

## Alternatives

- **A lock flag,** so a second `lock()` while a guard is alive would
  deadlock or panic as Rust's does. Rust's own behavior there is a hang,
  which no program wants, and the flag would cost every access, as
  ADR 0025 found for `RefCell`'s borrow flag.

## Consequences

- A program that locks twice on one thread, `m.lock()` while a guard of
  `m` is alive, hangs natively and doesn't in JS: nothing checks the lock,
  as nothing checks a `RefCell`'s borrows.
- `drop(guard)` of a guard, which has no destructor in JS, is nothing.

## Amendment: a lock holding a value with a destructor

A `Mutex` or an `RwLock` of a value with a destructor crashed the compiler:
its drop asked a cell, `{ value }`, for the fields of a struct. It drops
what it holds as a `RefCell` does (ADR 0320): when it's dropped, shared by
an `Arc` and dropped by its last, in a `Vec` or a struct; a write through
its guard drops the old value; and `into_inner` gives the value back,
dropped then by whoever has it. The `lock_destructors` corpus case runs
each against native Rust; mutations of the drop and of `into_inner` are
caught.

## Amendment: a static lock or atomic is one cell (#330)

A static `Mutex`, `RwLock` or atomic, std's way to keep global state, was
refused: "statics of type `RwLock<..>`". A static without `mut` is one
place every use reads, which changes only through interior mutability, so
**it's one JS cell made of its initializer**, as a `OnceLock` static is
(ADR 0317): `const TABLE = { value: Table.new() }`, which `$lock` and
`$lockRead` guard as a local's. TheAlgorithms/rust's Bell numbers keep
their lookup table so. Held by the corpus case `static_locks` and the
mutation `static-lock-refused`.


## Amendment: a lock a panic poisons is poisoned (#1438)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. The user's decision, reopening ADRs 0327 and 0449.

A lock was never poisoned, on the reasoning that nothing could catch the
panic that would poison it. `catch_unwind` does catch it, and so does JS
that calls an exported function again, or a drop a panic runs: there
`lock()` gave `Ok` where Rust's gives `Err(PoisonError)`, and
`is_poisoned()` was `false` where Rust's is `true`. That's a wrong answer.

**A lock is poisoned as std's is:**

| Rust | JS |
|---|---|
| a `MutexGuard` or `RwLockWriteGuard` dropped | `$unlock(guard)`: poisons its lock where a panic unwinds that didn't as it was taken |
| `m.lock()`, `l.write()`, `l.read()` | `$locked(m, true)`, `$locked(l)`: `Ok` of the guard, or `Err` of `{ data: guard }` |
| `.unwrap()`, `.expect(msg)` of one | `$lock(m)`, `$lockRead(l)`, `$lock(m, msg)`: the guard, or `unwrap`'s panic, `.. : PoisonError { .. }` |
| `into_inner()`, `get_mut()` | `$lockedValue(m)`; unwrapped, `$unpoisoned(m).value` |
| `try_lock()` of a poisoned lock | `Err` of `TryLockError::Poisoned` of the `PoisonError` |
| `is_poisoned()`, `clear_poison()` | `$isPoisoned(m)`, `$clearPoison(m)` |
| `{:?}`, `{}` of a `PoisonError`; `{:?}` of a lock | `PoisonError { .. }`, `poisoned lock: another task failed inside`; `poisoned: true` |

- **Poisoned where std's is:** a guard taken while no panic unwinds and
  dropped as one does. One taken in a drop a panic runs poisons nothing,
  as std's records `thread::panicking()` as it's taken; a reading guard
  never does. The runtime keeps which locks are poisoned in a `WeakSet`,
  so a lock is still `{ value }`.
- **Only a cleanup a panic runs knows one unwinds** (`$cleanup`, ADR
  1126): a `finally` runs either way. So a drop that asks, `$unlock` or
  `thread::panicking()`, is in no `finally` (`drops::finally_holds`): a
  guard's scope is one `try { .. } catch (error) { $cleanup(() =>
  $unlock(g), error); throw error; }` and the `$unlock(g)` after it, where
  it was `try { .. } finally { $unborrow(g); }`; and a failed bounds
  check's drops run in `$cleanup` before it throws.
- **A `LazyLock` whose init panics is poisoned for good**, each later use
  panicking `LazyLock instance has previously been poisoned`, as std's; it
  was refused as an init that used itself. A `OnceLock`'s init that panics
  leaves it empty, tried again, as std's.
- **`Once` is still refused**, so its poisoning is too.

Held by the corpus case `lock_poisoning`, against native Rust in both
builds, and by mutations of where a guard poisons, of what a poisoned
lock's `lock()`, `try_lock()` and `into_inner()` give, of `unwrap()`'s
panic, of `clear_poison()`, of a `PoisonError`'s `{:?}` and `{}`, of a
`LazyLock`'s poisoning, and of which drops a `finally` may hold.
