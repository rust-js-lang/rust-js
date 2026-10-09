# 0317. A `OnceCell` is a `{ value }` of an `Option`, set once

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md) and
[0051](0051-generic-options.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), but for `wait()`
and a `OnceLock`'s reentrant init, D: refused.

## Context

ADR 0314's count had `OnceCell` and `OnceLock` at none of their methods:
even `OnceCell::new()` and a `static` `OnceLock` were refused.

## Decision

**A `OnceCell` or `OnceLock` is a `{ value }` holding an `Option`, as a
`Cell<Option<T>>` is, and its methods are std's, by a helper each where
setting it must check it's unset.**

| Rust | JS |
|---|---|
| `OnceCell::new()`, `OnceLock::new()` | `{}` |
| `c.get()`, `c.into_inner()` | `c.value` |
| `c.set(x)` | `$onceSet(c, x)`, `Err(x)` if it's set |
| `c.get_or_init(f)` | `$getOrInit(c, f)`, which panics as std's does if `f` sets it; a `OnceLock`'s `$getOrInitLock(c, f)` |
| `c.take()` | `$cellReplace(c)` |
| `c.get_mut()` | `c.value` of an object; of a number or text, `c` itself, the `{ value }` a `&mut` to one is (ADR 0074) |
| `{:?}`, `clone()`, `==`, `default()` | `OnceCell(1)` or `OnceCell(<uninit>)`, and its `Option`'s |

- What it holds is `$some(x)`, boxed where it looks like `None` (ADR
  0051), so a `OnceCell<()>` that's set is set.
- A `static` `OnceLock`, which `new()` only ever makes empty, is its
  module's one `{}`, as a `static` is one value (ADR 0096).
- `OnceCell` and `OnceLock` have no diagnostic items: recognition knows
  them by their crate and name, `core`'s `OnceCell` and `std`'s `OnceLock`.
- A `OnceLock`'s `f` that sets it deadlocks in Rust, which JS can't do:
  `$getOrInitLock` throws `rust-js does not support ..` there, not
  `OnceCell`'s panic.
- `wait()` stays refused: on JS's one thread, nothing else can set it, so
  an unset one would wait forever.
- `get_mut()` of a value that looks like `None`, `()` or an `Option`, is
  refused: its box isn't the `{ value }` a `&mut` to one is.

## Why

- **It's the same program**: `get_or_init`'s `f` runs once, a second `set`
  is `Err`, and an init that sets the cell itself panics, as std's do.
- **It's tested**: a corpus case runs each against native Rust, of a
  number, text, an object, `()` and an `Option`, a `static` and `{:#?}`, and
  one the reentrant panic; mutations overwrite on a second `set`, unbox,
  run `f` every time, drop the panic, copy a `get_mut` number, keep what
  `take` took, and refuse a `OnceLock`, a `static` one and a `()` one.
- `docs/std-coverage.txt`: `OnceCell` 7 of 7, `OnceLock` 7 of 8.
