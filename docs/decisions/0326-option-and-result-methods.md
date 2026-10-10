# 0326. `Option`'s and `Result`'s other methods, and an `Option`'s place filled

Status: Accepted. Extends [0030](0030-option.md), [0051](0051-generic-options.md)
and [0099](0099-mut-references.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), A where JS's own is
the same.

## Context

ADR 0314's count had `Option` at 27 of its 46 methods and `Result` at 19
of 36: `and`, `xor`, `zip`, `transpose`, `inspect`, `get_or_insert` and
`take_if` of an `Option`, and `and`, `or`, `flatten`, `inspect_err` and
`cloned` of a `Result`, were refused.

## Decision

**Each is std's, by the `Option`'s value or `undefined` and the `Result`'s
`{ TAG, _0 }`: an expression where it only reads, and its place written
where it fills or takes from the `Option`.**

| Rust | JS |
|---|---|
| `a.and(b)`, `a.xor(b)`, `a.zip(b)`, `o.unzip()` | `a != null ? b : undefined`, the one that's there, `[a, b]` of both, a pair of halves |
| `o.transpose()`, `r.transpose()` | `Ok(None)` of a `None`, `Some(Ok(x))` of `Ok(Some(x))`, and the rest |
| `o.inspect(f)`, `r.inspect(f)`, `r.inspect_err(f)` | `if (o != null) f(o);`, then the value |
| `map_or_default(f)`, `o.as_slice()`, `r.iter()` | `f`'s, or `U`'s default; `[o]` or `[]` |
| `r.and(b)`, `r.or(b)`, `r.or_else(f)`, `r.flatten()` | by `r.TAG` |
| `r.cloned()`, `r.copied()`, `as_deref()` | a clone of the `Ok`'s value where a clone is more; the value |
| `unwrap_unchecked()`, `unwrap_err_unchecked()` | `unwrap()`'s, which panics where std's is undefined |
| `o.get_or_insert(v)`, `_with(f)`, `_default()` | `o ??= v`, then the `&mut` to it |
| `o.insert(v)`, `o.take_if(p)` | `o = v`; `if (o != null && p(o)) { taken = o; o = undefined; }` |

- **The `&mut` an `Option`'s place gives** is what it holds where that's
  an object, and for a number or text a handle on the place (ADR 0099),
  whose `value` is the place: `*count.get_or_insert(5) += 1` is `count ??=
  5; count = (count + 1) | 0;`. A handle on a place that reads the same is
  never put in a `const` first.
- **`b` is worked out either way**, as Rust's argument is: in a `const`
  first where it has effects.
- **`as_deref` is the value of a `String`, a `Vec`, a box or a
  reference**, as ADR 0211's `Option` of a `String` was; another type's
  own `Deref` is refused.
- **Refused, loud**: an `Option` whose value could look like `None`
  changed in place, one in a map changed in place, `as_mut_slice`,
  `iter_mut`, a `Result`'s `as_mut`, and the pinned ones.

## Why

- **It's the same program**: each reads, fills and takes as std's does.
- **It's tested**: a corpus case runs each against native Rust, of `Some`
  and `None`, `Ok` and `Err`, counted inspections, numbers and objects
  filled in place, and a predicate that takes or leaves. Mutations make
  `None.and(b)` `b`, keep both of `xor`, lose `Ok(None)`, inspect a
  `None`, swap `and` and `or`, leave `flatten` nested, overwrite with
  `get_or_insert`, take without asking, write a number's field, and spill
  a handle.
- `docs/std-coverage.txt`: `Option` 40 of 46, `Result` 32 of 36.

## Since

- **`as_deref` of an `Rc`, an `Arc` or a type with its own `Deref`**
  (2026-10-10), which were refused. One not counted is what it points at
  (ADR 0023), so the `Option` or `Result` is itself, as a `String`'s; a
  counted one's is its `value` (ADR 0320), one layer of it; another type's
  is its own `deref()`, called only on a `Some` or an `Ok`.

  ```rust
  let named = Some(Name("custom".to_string()));
  println!("{:?}", named.as_deref());
  ```

  ```js
  const arg$3 = named != null ? nameDeref_deref(named) : undefined;
  ```

  An `Option` of what could look like `None` stays refused. The
  `as_deref` corpus case runs, against native Rust, `String`s matched and
  mapped, a `Vec`'s slice, a box, an `Rc` not counted, a type's own
  `Deref` that prints, and `Ok`s and `Err`s; `as_deref_counted` counted
  ones, two layers of them and a `Result` of one; `as_deref_none_like`
  the refusal. Mutations give the `Rc` for its `value`, unwrap the
  `Result`, and refuse each. `docs/std-coverage.txt`: `Option` 42 of 46,
  `Result` 35 of 36.
