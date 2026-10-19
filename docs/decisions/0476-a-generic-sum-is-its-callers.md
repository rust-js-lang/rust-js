# 0476. A generic `sum` is its caller's

Status: Accepted. Extends [0445](0445-extend-and-collect-in-generic-code.md),
[0160](0160-user-collections.md) and [0011](0011-numbers.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same value, made
by the same impl.

## Context

`sum()` and `product()` were of a number rustc knows, a `reduce` by its
operation, or of the crate's own type's impl, called as it is. Into a type
parameter, `sum::<S>()` of an `S: Sum<Self::Item>`, itertools' `sum1` and
`product1`, they were refused. So was `collect()` into what holds one that a
bound says it's collected into, `Result<U, E>: FromIterator<Result<T, E>>`,
itertools' `try_collect`: only a bare `B: FromIterator` was (ADR 0445).

## Decision

- **`sum()` into an `S: Sum<A>`, and `product()` into a `P: Product<A>`, is
  its dictionary's**, `SSum.sum(it)`, its caller's: `Sum` and `Product` are
  `operational`. Of a number, std's dictionary is `{ sum(items) }` of the
  same `reduce` concrete code makes, `accumulated_numbers`; of the crate's
  own, its impl's, written only where generic code reads it
  (`read_only_generically`).
- **`collect()` into what holds a type parameter a bound says it's
  collected into**, a `B` or a `Result<U, E>`, is its dictionary's
  `from_iter`, its caller's, before anything else: one way for both.

```js
function sum1(it, SSumIItem) {
  const first = $nextSome(it);
  return first != null ? $some(SSumIItem.sum($lazyChain($iter([$someValue(first)]), it))) : undefined;
}

total([200, 100], { sum: (items) => Iterator.from(items).reduce((a, b) => (a + b) >>> 0, 0) });
```

## Why

- **It's std's**: `generic_accumulations` sums and multiplies into an
  `i32`, an `f64` (from -0.0) and a `u64`, wrapping as ADR 0011 does, and
  collects `Result`s, through generic code, with native Rust.

## Consequences

- **What compiles grows**: `sum()` and `product()` into a type parameter,
  and `collect()` into a `Result<U, E>` a bound names.
