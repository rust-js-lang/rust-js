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

## Amendment: a generic function's instance is one function (#974)

A generic function taken as a value is the arrow that gives it its
dictionaries, `(arg0) => noop(arg0, tMetamorphGetT)`, which was made again
at each use: `std::ptr::eq(handler, &noop)` and `f == table[0]` of
`double::<i32>` taken twice were false where Rust's are true. An instance
its module knows whole, its arguments of no type parameter, is its module's
`const`, made the first time it's taken and named for the function and its
arguments, `const noopU32 = (arg0) => noop(arg0, tMetamorphGetT)`,
`count4`, `hereValue` of a `#[track_caller]` one given its location alone
(`module_fn_value`, `Kind::FnValue`, as std's order dictionaries are
`const`s of their module, #944): each use is the one function. Its drop
arguments are settled as an item's are (ADR 0300); where its drops leave the
function itself, `keep`, each use reads the function, and one nothing reads
is never written. Rust's answer is kept (ADR 0262's third case). An
instance given its caller's dictionaries, taken in generic code, is still
the arrow made where it's taken: one function for each time its maker
runs. Corpus case `generic_fn_value_is_one_function`; the metamorphic net's
`fn_pointer_eq` and `fn_compared_by_its_pointer` rewrites run as native.
