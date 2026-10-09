# 0318. A `LazyCell` makes its value when it's first used, as Rust's does

Status: Accepted. Extends [0096](0096-statics.md) and
[0317](0317-once-cells.md); counted by [0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), but for a
`LazyLock`'s reentrant init, D: refused.

## Context

ADR 0314's count had `LazyCell` and `LazyLock` at none of their methods,
and a `static` `LazyLock`, Rust's way to a table made once, was refused.
JS could make it at load, `const TABLE = new Map(..)`, which reads as
written by hand; but Rust makes it when it's first used, so what its init
prints, or a panic in it, happens then, and an unused one never runs.

## Decision

**A `LazyCell` or `LazyLock` is `{ init }`, whose `$force` runs `init`
the first time its value is used, and keeps the `value` it makes.**

```rust
static TABLE: LazyLock<HashMap<&str, u32>> = LazyLock::new(|| HashMap::from([("one", 1)]));
fn lookup(name: &str) -> Option<u32> { TABLE.get(name).copied() }
```

```js
const TABLE = { init: () => new Map([["one", 1]]) };
function lookup(name) {
  return $forceLock(TABLE).get(name);
}
```

| Rust | JS |
|---|---|
| `LazyCell::new(f)`, `LazyLock::new(f)` | `{ init: f }` |
| `*x`, `x.method()`, `LazyCell::force(&x)` | `$force(x)`, a `LazyLock`'s `$forceLock(x)` |
| `*x = ..`, `force_mut` of a number or text | `$force(x)`, then `x`, the `{ value }` a `&mut` to one is (ADR 0074) |
| `LazyCell::get(&x)`, `get_mut` | `x.init === undefined ? x.value : undefined` |
| `{:?}`, `default()` | `LazyCell(1)` or `LazyCell(<uninit>)`, and `{ init: () => T::default() }` |

- While `init` runs, and after it panics, `init` is `null`: std's
  poisoned. An init that uses its own `LazyCell` panics with std's
  message. One that uses its own `LazyLock` deadlocks in Rust, which JS
  can't do: `$forceLock` throws `rust-js does not support ..` there.
- A `static` one is its module's one `{ init }` (ADR 0096): the init is a
  function, which runs nothing as the module loads.
- `get_mut` of a value that looks like `None` is refused, as a
  `OnceCell`'s is; `From<T>` is refused.

## Why

- **It's the same program**: the init runs when Rust's does, once, or
  never if it's never used.
- **It's tested**: a corpus case runs each against native Rust, `static`
  ones read through `Deref` before and after other output, numbers changed
  through `DerefMut`, `force_mut` and `get_mut`, a `Vec` pushed to, `()`,
  `default()` and `{:#?}`, and one the reentrant panic; a runtime test a
  `LazyLock`'s and a `OnceLock`'s reentrant inits; mutations run init
  every time, drop the poisoning, make `default()` eagerly, read an unmade
  value, copy a number given as `&mut`, and refuse a `LazyLock` and a
  `static` one.
- `docs/std-coverage.txt`: `LazyCell` 5 of 5, `LazyLock` 5 of 5.
