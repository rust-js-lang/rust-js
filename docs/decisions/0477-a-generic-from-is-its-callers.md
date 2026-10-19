# 0477. A generic `From::from` is its caller's

Status: Accepted. Extends [0108](0108-generic-operators-and-into.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same value, made
by the same conversion.

## Context

`x.into()` of a `T: Into<U>` is its dictionary's `into`, std's conversion
or the crate's `From` impl (ADR 0108). `W::from(u)` of a `W: From<U>` was
refused, `From` being a trait whose impls rust-js lowers but whose bounds it
gave no dictionary. itertools calls one of an associated type a trait bounds
by it, `T::Combination::from(iter)` of `type Combination: From<I>`, and of
what a bound names, `Zip::from(t)` of `Zip<T>: From<U>`.

## Decision

**`W::from(u)` of a `W: From<U>` is its dictionary's `from`**, its
caller's: `From` is `operational`, and its dictionary `{ from }` is made as
`Into`'s `{ into }` is, std's conversion or the crate's impl's function. An
associated type's bound by it is its trait's dictionary's, as any of its
bounds is. Where the types are known, or no bound gives it, `VecDeque::from(v)`
of a `Vec<T>`, std's impl wherever it's called, it's written in place, as
ever, and a crate's impl's dictionary is written only where generic code
reads it (`read_only_generically`).

```js
function build(u, WFromU) {
  return WFromU.from(u);
}

build([3], { from: wrapFromVec_from });
conv(7, { from: (n) => n });
```

## Why

- **It's the same conversion**: `generic_from` converts by the crate's
  impl, by an associated type's bound, and by std's, `u32: From<u8>` and
  `String: From<&str>`, through generic code, with native Rust.

## Consequences

- **What compiles grows**: `From::from` through a bound.
