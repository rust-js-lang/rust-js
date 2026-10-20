# 0490. A value std leaves unused is dropped there

Status: Accepted. Extends [0469](0469-a-function-std-leaves-unrun-is-dropped-there.md)
and [0179](0179-option-filter-map-or-drops.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

A function std is given and leaves unrun is dropped where it's left, on
its subject's variant (ADR 0469). A value it's given and leaves unused
wasn't: `Some(a).unwrap_or(b)` drops `b`, `x.ok_or(e)` and `x.or(y)` of a
`Some` drop theirs, `false.then_some(v)` drops `v`, and each was refused,
"of a value with a destructor". A `Result`'s `unwrap_or(d)` and `or(r)`
give up an `Err`'s value too, which std drops.

## Decision

**A value std is given and leaves unused is dropped as a function it
leaves unrun is: one table, `unused_arguments`, says on which variant of
its subject each argument is unused, a function or a value; `Unused`
names the variant.** Held in a variable, the value is dropped after the
call where its subject is that variant.

- `unwrap_or(d)`, `ok_or(e)` and `or(o)` of an `Option`: of a `Some`.
- `unwrap_or(d)` and `or(r)` of a `Result`: of an `Ok`; and the `Err`'s
  value they give up dropped where they give it up, as `ok()`'s (ADR
  0179), by one helper, `kept_or`, beside `or_dropped`.
- `then_some(v)`: of `false`, as `then(f)`'s function.
- `map_or(d, f)`'s default isn't in the table: its lowering drops it
  already (ADR 0179).

```js
function pick(o, d, dropT) {
  const value = $someValue(o ?? $some(d));
  if (o != null) {
    dropT?.(d);
  }
  return value;
}
```

## Why

- **It's std's**: `unused_value_drops` gives each of them values with a
  destructor, of each variant, a generic `T`'s by its given drop too, and
  `Result`s whose `Err` has one, with native Rust.
- **One way**: a function unrun and a value unkept are one question, which
  argument std drops on which variant, and one table answers it.

## Consequences

- **What compiles grows**: `unwrap_or`, `ok_or`, `or` and `then_some` of
  a value with a destructor, and a `Result`'s `unwrap_or` and `or` of one
  whose `Err` has one.
- **`and` and `xor`**, which drop the subject's own value, are refused
  yet.
