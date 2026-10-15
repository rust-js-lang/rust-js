# 0387. A conversion of a type to itself is the value

Status: Accepted. Extends [0108](0108-generic-operators-and-into.md) and
[0063](0063-text.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): std's `From<T> for T`
gives back what it's given, which JS's value is already.

## Context

std has `impl<T> From<T> for T`, and the `Into` every `From` gives. rust-js
knew it only of the types it knew another conversion of, numbers and
strings, and refused it of any other. Two of rustc's tests failed for that:

```rust
// `try!(e)`, and `?`, convert an error by `From`, here to its own type.
fn simple() -> Result<i32, ParseIntError> {
    Ok(try!("1".parse()))
}

// An `impl Into<[T; 1]>` that is a `[T; 1]`.
fn foo<T>(t: T) -> impl Into<[T; 1]> { [t] }
println!("{:?}", foo(42).into());
```

and a third, of an `impl Into<fn(&())>`.

## Decision

**`From::from(x)` and `x.into()` of a type to itself, as rustc sees the
types once an `impl Trait` is the type it stands for
([0061](0061-generic-iterators.md)), are `x`, whatever the type.**

```js
function kept(error) {
  return error;
}
```

- **The one place that says so is recognition**: a conversion of a type to
  itself is std's `Same`, there, before any other of its conversions, for
  a call and a generic function's dictionary alike. The dictionary's own
  test of the same, which only a type std knew no conversion of reached, is
  gone: its function, `(value) => value`, is std's `(x) => x` now.
- **No crate can make it otherwise**: coherence refuses an
  `impl From<X> for X` beside std's.

## Why

- **It's what Rust does**: std's `from` is `t`.
- **It's tested**: the corpus's `into_itself.rs` converts a parse error to
  its own type, through `?` too, and an `impl Into` of a struct and of an
  array; `generic_into.rs` gives a struct to itself through a dictionary;
  and the three rustc tests pass.
