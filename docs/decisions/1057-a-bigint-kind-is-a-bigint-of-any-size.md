# 1057. A `bigint` `classify` tells is a `js::BigInt`, of any size

Status: Accepted (rd512, P1, 2026-11-02). Amends
[0225](0225-unknown-values.md) (`classify` tells a value by `typeof`) and
[0086](0086-64-bit-integers.md) (an `i64` is a `bigint`).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. An `i64` holds 64 bits and wraps past ±2^63. A
JS `bigint` may be of any size. Reading one as an `i64` gave Rust a value
no `i64` can be: `n + 1` wrapped it, and `n as f64` read a number the Rust
said was smaller. Now no `bigint` is read as an `i64`.

## Context

React's Flight reply client sends a `bigint` as its digits,
`'$n' + value.toString(10)`. The reply server estimates the digits of
one it reads, `Math.abs(Number(value))`. A `bigint` there may be wider
than 64 bits. `js::Kind::BigInt` held an `i64`, so the port couldn't
write either:

- `i64::to_string()` is Rust's formatting of a value that may not fit;
- `value as f64` is a cast of one;
- any arithmetic on it wraps at 2^63 (ADR 0086), where JS's doesn't.

`js::BigInt`, the js crate's `bigint` of any size (rd399), had no
`toString`.

## Decision

**`Kind::BigInt` holds a `&js::BigInt`, the value itself, of any size.
`BigInt::to_string(radix)` is `n.toString(radix)`, `None` of ten, and
`js::number::from_big_int(n)` is `Number(n)`.**

```rust
match js::classify(value) {
    Kind::BigInt(n) => format!("$n{}", n.to_string(Some(10))),
    ..
}
```

```js
case "bigint":
  return `$n${value.toString(10)}`;
```

The `switch (typeof value)` is as it was. The variant is still told by
`typeof`: `BigInt` is typed `bigint` (`untagged_kind`, by its TypeScript
type).

## Alternatives

- **Keep the `i64` and refuse arithmetic on it.** Rejected: rustc owns
  `i64`'s operations, and `as f64` and `to_string` are already results no
  JS `bigint` past 64 bits would give.
- **`Display` for `BigInt`**, `format!("{n}")`. Not now: it would sit beside
  `to_string(radix)`. The inherent method would shadow `ToString`, so
  `n.to_string()` wouldn't compile. `js::shown(n)` is a template's.

## Consequences

- A program that read the `i64`, `Kind::BigInt(n) => n + 1`, no longer
  compiles. It converts on purpose: `js::number::from_big_int(n)`, or
  `n.to_string(None).parse::<i64>()` where it means an `i64`.
- React's ports: the reply server's `(value as f64).abs()` is
  `js::number::from_big_int(value).abs()`, upstream's
  `Math.abs(Number(value))` (rd489). The reply client writes
  `value.to_string(Some(10))`.
- Held by `test/js-any-values.test.ts` ("a bigint classify tells is one of
  any size, its toString and Number"). It checks a `bigint` of 2^70, one of
  -12, `toString(16)`, and one of 10^400 whose `Number` is `Infinity`.
  Bindings have no mutations: what each says is its JS, and the test checks
  it.
