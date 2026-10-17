# 0428. A C-variadic function's `...` is JS's rest parameter

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what a `VaList` reads
is what it was given, in order, as Rust's does.

## Context

A C-variadic function, `unsafe extern "C" fn f(n: i32, mut args: ...)`,
takes any arguments past the rest, which it reads in order through its
`VaList`, `args.next_arg::<i64>()`, and may copy, `args.clone()`, to read
again. It was an error, at `next_arg`, which stopped four of rustc's tests.
JS has the same: a rest parameter, `...args`, the array of what's given
past the rest.

## Decision

**A C-variadic function's last parameter is a rest parameter, `...args`:
its `VaList` is the array of what it hasn't read yet.**

```rust
unsafe extern "C" fn sum(count: i32, mut args: ...) -> i64 {
    let mut total = 0i64;
    for _ in 0..count {
        total += unsafe { args.next_arg::<i64>() };
    }
    total
}
```

```js
function sum(count, ...args) {
  let total = 0n;
  for (let i = 0; i < count; i++) {
    total = BigInt.asIntN(64, total + args.shift());
  }
  return total;
}
```

- **`next_arg()` is `shift()`,** the first of what's left; a clone is
  `slice()`, what's left, read on its own; a call gives what it's given as
  it is, and through a function pointer too.
- **Which functions are is `recognition::is_c_variadic`'s**, rustc's
  signature's own, and `mir_function`, which makes every body's
  parameters, a method's and a trait default's alike, makes its last a
  rest. `js::Pattern::Rest` is the parameter, written as JS's.
- **A trait's C-variadic method as a value** is `(arg0, ...args) =>
  dictionary.method(arg0, ...args)`.
- **Refused:** one given evidence, a dictionary, a type's fact, a drop or
  its caller's location (ADR 0427), which a rest parameter can't come
  before; and a trait's C-variadic method of a `&mut` to an object, as a
  value, whose arrow has no rest yet.

## Why

- **It's exact:** the `c_variadic` corpus case sums what it's given, reads
  a clone and the original apart, of a method too, and through a function
  pointer, with native Rust; rustc's `c-variadic/valid`, `inherent-method`
  and `copy` pass.
- **It's the JS a person writes:** a rest parameter, and `shift()`.

## Alternatives

- **A `$iter` of the arguments, `$next(args)`:** a helper where JS has the
  array and its own methods.

## Consequences

- rustc's `c-variadic/trait-method` takes a trait's C-variadic `&mut self`
  method as a function pointer, refused. (Amended: the arrow that boxes
  its `&mut` gathers the rest too, `(x, ...args) => ..`, and passes it on,
  as a call's operands past its signature are passed; it passes.)
