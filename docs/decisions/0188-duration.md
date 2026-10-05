# 0188. A `Duration` is its nanoseconds, a BigInt

Status: Accepted. Extends [0011](0011-numbers.md), [0086](0086-64-bit-integers.md)
and [0177](0177-nonzero.md).

## Context

chrono's `TimeDelta` converts to and from std's `Duration`, and adds one
to a time, so chrono stopped at `Duration::new`, `as_secs()` and
`subsec_nanos()`. std keeps a `Duration` as a `u64` of seconds and a `u32`
of nanoseconds under one, which rust-js had no value for.

## Decision

**A `Duration` is its nanoseconds, a BigInt, as a `u128` is: `1.5s` is
`1500000000n`.** Its methods are of that number:

| Rust | JS |
|---|---|
| `Duration::new(5, 1_500_000_000)` | `$durationNew(5n, 1500000000)`, which panics past `MAX` |
| `Duration::from_secs(90)` | `90n * 1000000000n` |
| `d.as_secs()`, `d.as_millis()` | `d / 1000000000n`, `d / 1000000n` |
| `d.subsec_nanos()`, `d.subsec_millis()` | `Number(d % 1000000000n)`, `Number((d % 1000000000n) / 1000000n)` |
| `Duration::ZERO`, `Duration::MAX` | `0n`, `18446744073709551615999999999n` |
| `a + b`, `a - b` | `$durationAdd(a, b)`, `$durationSub(a, b)`, which panic as std's do |
| `a.checked_add(b)`, `a.checked_sub(b)` | `None` where they would |
| `==`, `<`, `cmp`, `sort()`, a `HashSet`'s keys | its number's |
| `{:?}` | `$debugDuration(d)`: `1.25s`, `800ms`, `3µs`, `0ns` |

- **Its number in every way that's its own:** `Num::of` takes it for a
  `u128`, as it takes a `NonZero` for its number (ADR 0177), so it's
  compared, sorted, hashed and copied as one.
- **But not its number's arithmetic:** `+` and `-` panic past `MAX` and
  below zero, as std's do, and its other operators, `*` and `/` of a
  `u32`, are an error for now.
- **`{:?}` with options, `{:.2?}`, is an error for now:** std rounds and
  pads one by rules of its own.

## Why

- **It's exact:** the `duration` corpus case makes one each way, with
  nanoseconds that carry into seconds, asks it each part, adds, subtracts
  and checks, compares, sorts, hashes and shows it in each unit, with
  native Rust, and `duration_overflow` panics as std's does.
- **It's the JS a person writes:** a count of nanoseconds, compared with
  `<`, where std's two fields would be compared one after the other.

## Alternatives

- **std's two fields, `{ secs, nanos }`:** the same answers, but `==` and
  `<` of two would each be a call, and a `HashSet` of them would key by
  identity.

## Costs

- **Every operation is of a BigInt,** as a `u64`'s is (ADR 0086).
- **`*`, `/`, `+=`, `-=`, `sum()` and the other methods are errors** until
  something needs them.
