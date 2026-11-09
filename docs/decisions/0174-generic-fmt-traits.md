# 0174. `{:x}` and `{:p}` of a generic `T`: its dictionary's

Status: Accepted. Extends [0165](0165-other-fmt-traits.md) and
[0049](0049-traits-and-generics.md).

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0165 called a type's own `LowerHex` or `Pointer` where its type is
known. Generic code forwards them, as thiserror's `Var` does for the
`{:p}` its derive writes:

```rust
impl<'a, T: Pointer + ?Sized> Pointer for Var<'a, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Pointer::fmt(self.0, formatter)
    }
}
```

`Pointer::fmt` of a `T` was an error, which stopped thiserror.

## Decision

**`LowerHex`, `UpperHex`, `Octal`, `Binary`, `LowerExp`, `UpperExp` and
`Pointer` are traits of dictionaries, `{ fmt }`, as `Display` is:** a
generic function takes `T`'s, and `{:x}` of a `T`, or `LowerHex::fmt(t,
f)`, is its `fmt`, given the placeholder's options as a `T: Display`'s is
(ADR 0137).

```js
function hex(t, TLowerHex) {
  return `${TLowerHex.fmt(t)}|${TLowerHex.fmt(t, { alternate: true })}`;
}
```

- **The crate's is its impl's,** `handleLowerHex()`.
- **Still errors:** std's, of a number given where a `T: LowerHex` goes,
  whose `fmt` would apply the options itself, and of a reference, whose
  address JS hasn't (ADR 0165). (Amended: a number's is a dictionary that
  applies them as it runs, ADR 0185.)

## Why

- **It's exact:** the `generic_fmt_traits` corpus case forwards `{:p}`,
  `{:x}` and `{:#x}` through thiserror's `Var` shape and a generic
  function, with native Rust.
- **It's the JS a person writes:** the formatter given to the generic
  function, as its `Display`'s is.

## Amendment: a bound a caller gives, and a path call of `fmt` (#1115)

`{:x}` of a value was its caller's dictionary's only where its type was a
type parameter, `is_unknown`'s. Of one whose impl no resolution here picks
but a bound the function is given covers, libm's `impl<T> fmt::Debug for
Hexf<T> where Hexf<T>: fmt::LowerHex`, whose impls are `Hexf<u8>`'s and its
tuples', it was refused, and so was the same `fmt` called by its path,
`fmt::LowerHex::fmt(self, f)`. libm was types alone, so num-traits with
`libm` and each crate implementing a num-traits trait, chrono's library in
its test build among them.

`formatted_by_dictionary` is the one question both ask now: a type only a
caller knows, or one with no impl here a given bound covers, is formatted
by that bound's dictionary, the `Formatter`'s options passed on. A path call
of any formatting trait's `fmt`, `Display`'s, `Debug`'s, the radix traits',
`LowerExp`'s and `Pointer`'s, already went where `{}`, `{:?}` and `{:x}` of
its value go, std's impls of numbers as their digits, a user impl by its
own `fmt`; a bound's now too. `test/corpus/fmt_trait_fmt_called_by_path.rs`
holds each against native Rust: std's impls by path under `{:#06}`, a user
impl reading `f.alternate()`, libm's generic shape, and an error a writer
gives through the callee; the mutation `fmt-by-a-given-bound-unasked` puts
the old question back.
