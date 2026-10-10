# 0343. A `&mut` to what's in an `Option` or a `Result`

Status: Accepted. Extends [0099](0099-mut-references.md) and
[0030](0030-option.md); counted by [0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`iter_mut()` of an `Option` or a `Result`, `for x in &mut option`, and a
`Result`'s `as_mut()` give a `&mut` to what's in it. Each was an error:
of a number or text, the `&mut` must write the `Option` or the `Result`
it's in, which a copy of the value can't.

## Decision

**Of a number or text, the `&mut` is a handle on where it is: an
`Option`'s own place, as `Some(x)` of one is `x` (ADR 0030), or a
`Result`'s `_0`. Of an object, the object.**

```rust
for c in count.iter_mut() {
    *c += 1;
}
if let Ok(x) = ok.as_mut() {
    *x *= 5;
}
```

```js
for (const c of count == null ? [] : [{ get value() { return count; }, set value(value) { count = value; } }]) {
  c.value = (c.value + 1) | 0;
}
const x = { TAG: ok.TAG, _0: { get value() { return ok._0; }, set value(value) { ok._0 = value; } } };
if (x.TAG === "Ok") {
  x._0.value = Math.imul(x._0.value, 5);
}
```

| Rust | JS |
|---|---|
| `option.iter_mut()`, `for x in &mut option` | `[]` of `None`, else `[handle]` on the option's place, or `[object]` |
| `result.iter_mut()` | `[handle on r._0]` of `Ok`, else `[]` |
| `result.as_mut()` | `{ TAG: r.TAG, _0: handle }` where both sides are numbers or text; of one, a branch; of objects, `r` |

- **Still refused**: one of an `Option` of an `Option`, whose `Some` is
  boxed (ADR 0051), and `Option::as_mut_slice`.

## Why

- **It's the same program**: each write through the `&mut` is the
  `Option`'s or the `Result`'s, as Rust's is.
- **It's tested**: the `option_result_mut` corpus case runs, against
  native Rust, `iter_mut()` and `for .. in &mut` of `Some` and `None` of a
  number, a `String` and a struct, and a `Result`'s `as_mut()` and
  `iter_mut()` of a number, a `String`, and a number and a struct. Mutations
  give copies for handles, a cell for an object, `Ok` or `Err` always, and
  refuse each.
- `docs/std-coverage.txt`: `Option` 41 of 46, `Result` 34 of 36.
