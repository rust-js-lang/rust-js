# 0174. `{:x}` and `{:p}` of a generic `T`: its dictionary's

Status: Accepted. Extends [0165](0165-other-fmt-traits.md) and
[0049](0049-traits-and-generics.md).

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
  address JS hasn't (ADR 0165).

## Why

- **It's exact:** the `generic_fmt_traits` corpus case forwards `{:p}`,
  `{:x}` and `{:#x}` through thiserror's `Var` shape and a generic
  function, with native Rust.
- **It's the JS a person writes:** the formatter given to the generic
  function, as its `Display`'s is.
