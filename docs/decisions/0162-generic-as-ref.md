# 0162. `AsRef` in generic code: a dictionary, the value itself for std's

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0161](0161-generic-from-str.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An API that takes "anything that's text" or "anything that's a slice" is
generic over `AsRef`:

```rust
fn shout<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_uppercase()
}
```

`as_ref()` of a generic `S`, and a type's own `impl AsRef`, were errors.

## Decision

**`AsRef` is a trait of dictionaries, `{ as_ref }`,** as `FromStr` is (ADR
0161): a generic function takes `S`'s, and `s.as_ref()` in it is the
dictionary's.

```js
function shout(s, SAsRefStr) {
  return SAsRefStr.as_ref(s).toUpperCase();
}

shout("hi", { as_ref: (value) => value });
shout({ first: "ada" }, nameAsRefStr());
```

- **std's is the value itself** where `as_ref` changes nothing in JS: a
  `String` or a `&str` as a `str`, and a `Vec`, an array or a slice as a
  slice. Others, `AsRef<Path>` say, are errors.
- **The crate's is its impl's,** `nameAsRefStr()`.

## Why

- **It's exact:** the `generic_as_ref` corpus case compares `&str`,
  `&String`, `String`, an array, a `Vec`, a slice and the crate's types,
  through generic functions and directly, with native Rust.
- **It's the JS a person writes:** the conversion given to the generic
  function, the identity where there's nothing to convert.
