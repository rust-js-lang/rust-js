# 0162. `AsRef` and `AsMut` in generic code: a dictionary, the value itself for std's

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

## Amendment: `AsMut`

`as_mut()` of a generic `T: AsMut<str>` or `T: AsMut<[U]>`, as either's
`Either<L, R>` does of each side, was refused. `AsMut` is a trait of
dictionaries too, `{ as_mut }`, given the cell a `&mut self` is in
generic code (ADR 0099). Case N: the same text and items, changed in
place.

```js
function shout(t, TAsMutStr) {
  const part = TAsMutStr.as_mut({ get value() { return t; }, set value(value) { t = value; } });
  part.value = $asciiCase(part.value, true);
  return t;
}

shout("hi", { as_mut: (cell) => cell });
```

- **std's** is the cell it's given, as the `&mut T` of `as_mut`'s
  signature is a box in generic code (ADR 0454): a `&mut str` is its
  text's cell (ADR 0334), and a `&mut [T]`'s box holds its array, which
  a caller that knows it's a slice takes out, `.value`. Of a `&mut` to
  one, std's `impl AsMut<U> for &mut T`, one `value` further for each
  `&mut`. Others are errors. Whether std's `AsRef` or `AsMut` is the
  same JS value is one question, `views_as_itself`, which both ask.
- **The crate's is its impl's**, written only where generic code reads it
  (`read_only_generically`), as an operator's is: where the type is
  known, its method is called as it is.
- The `as_mut_generic` corpus case gives a `String`, a `Vec`, an array, a
  `&mut String` and a `&mut Vec` to generic functions, against native
  Rust; `user_as_mut` holds that a crate's impl writes no dictionary
  where nothing generic reads it.

## Amendment: text as `AsRef<[u8]>`

base64's `decode<T: AsRef<[u8]>>(input: T)` is given text. Its
dictionary's `as_ref` is the text's UTF-8 bytes, as `as_bytes()` gives
them, `utf8_bytes`, the one place both are made.
`test/corpus/as_ref_bytes.rs` holds it.

## Amended by

[ADR 0698](0698-a-reference-given-to-generic-code.md): a reference's `AsRef` is what it refers to's, and whether std's `AsRef` is the value is `as_ref_is_itself`'s, a path's as a `Path` too.
