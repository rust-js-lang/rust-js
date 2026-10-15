# 0395. Function pointers compare by identity

Status: Accepted. Extends [0053](0053-partial-eq.md).

Case: B ([0262](0262-when-rust-and-js-disagree.md)): JavaScript's, where
it differs rarely and harmlessly.

## Context

`f == g` of two function pointers compares their addresses in Rust. MIR's
own comparison of two was written `===`, but `PartialEq`'s, of references
to them or of an `Option` of one, was refused, and so rustc's
`function-pointer/function-pointer-comparison-54696.rs`.

## Decision

**A function pointer is compared as JS compares functions, by identity,
`===`, through `PartialEq` as by MIR's own comparison.**

```js
console.log(`${f === table[0]} ${g == square}`);
```

## Why

- **It's Rust's answer where Rust gives one**: the same function's pointers
  are equal, different functions' aren't.
- **It's tested**: the corpus's `fn_pointer_eq.rs` compares them, through
  references and in an `Option`, finds one in a table, with native Rust;
  the rustc test passes.

## Consequences

- **Where Rust's answer is unspecified, it's JS's**: rustc may give one
  function two addresses, across codegen units, or two functions of the
  same code one, which `unpredictable_function_pointer_comparisons` warns
  of. JS never does either: one function is equal to itself alone.
- `{:?}` of a function pointer, its address, is still an error: it changes
  from run to run.
