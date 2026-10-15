# 0410. An impl's constant of a function is lowered as code

Status: Accepted. Extends [0031](0031-consts.md),
[0047](0047-methods.md) and [0176](0176-generic-impl-consts.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): one JS function is
every use of the constant, as Rust's one function pointer is; nothing can
tell them apart.

## Context

A free constant whose value rustc's value tree can't say, a function's,
a closure's or a `dyn`'s, is lowered as code, a constant of its module
(ADR 0096). An impl's associated constant of one,
`const ID: fn(&S<T>) -> &S<T> = |s| s;`, `const TABLE: [(&str, fn(i32) ->
i32); 2]`, or a trait impl's `const ASSOC: fn(u32) -> u32`, was refused:
its initializer was never lowered, and a trait's constant was looked up by
the trait's item, not its impl's.

## Decision

**An impl's associated constant whose type has a function, a closure or a
`dyn` in it is lowered as code, once: an inherent impl's is a property of
its type's object, `S.ID`, before its methods, as a hand-written object of
functions has them; a trait impl's is a constant of its module, which its
dictionary reads. A trait's constant of a type is its impl's.**

```js
export const S = {
  ID: (s) => s,
  id(s) {
    return S.ID(s);
  },
};
const ASSOC = (x) => (x + 1) >>> 0;
```

- **Only what the value tree can't say**: any other impl constant is
  still its value where it's used, `S.ZERO` of `0` is `0`, as before.
- **A generic trait impl's** stays its dictionary's, of its parameters'
  evidence (ADR 0176); a trait's default, its `Self`'s.
- **A library's dictionary** reads the module's constant, `get ASSOC() {
  return ASSOC; }`, where it read a generic impl's initializer.

## Why

- **It's the program Rust runs**: the `impl_fn_consts` corpus case calls
  an inherent impl's closure constant directly and through a method, a
  trait's through a type and through generic code, a generic impl's of its
  parameter's `Default`, and a table of named functions, each with native
  Rust.
- Three of rustc's tests pass: `mismatched_impl_ty_2.rs`,
  `mismatched_impl_ty_3.rs`, `issue-58435-ice-with-assoc-const.rs`.

## Consequences

- A trait's default constant of a function, `<()>::vftable`, is still
  refused where it's read concretely, and a generic impl's constant made of
  std's `UnsafeCell`.
