# 0358. A number from a `bool` or a `char` is its cast

Status: Accepted.

## Context

std has `From<bool>` for every integer and float, and `From<char>` for
`u32`, `u64` and `u128`. rust-js refused them, `usize::from(flag)` among
them, though it writes `flag as usize` and `c as u32` already.

## Decision

**`T::from(x)` and `x.into()` of a `bool` or a `char` are what `as` makes of
them**, as a call and as a function, `.map(i8::from)`:

```js
flag ? 1 : 0     // usize::from(flag)
flag ? 1n : 0n   // u64::from(flag), a BigInt (ADR 0086)
c.codePointAt(0) // u32::from(c)
```

A float from a `bool`, which `as` has no form of, is `flag ? 1 : 0` too.
