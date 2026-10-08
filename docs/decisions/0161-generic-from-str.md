# 0161. `T: FromStr` in generic code: a dictionary, std's or the crate's

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0159](0159-user-from-str.md).

Case: N, A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Code that reads a value of any type from text is generic over `FromStr`:

```rust
fn read<T: FromStr>(s: &str) -> Option<T> {
    s.parse().ok()
}
```

`parse` of a `T` was an error: `FromStr` wasn't a trait whose dictionary
generic code is given (ADR 0049).

## Decision

**`FromStr` is a trait of dictionaries, `{ from_str }`:** a generic
function takes `T`'s after its arguments, and `s.parse::<T>()` in it is
the dictionary's `from_str`.

```js
function read(s, TFromStr) {
  const result = TFromStr.from_str(s);
  return result.TAG === "Ok" ? $some(result._0) : undefined;
}

read("42", { from_str: (s) => $parseInt(s, -2147483648, 2147483647) });
read("kg", unitFromStr());
```

- **std's is what `s.parse()` of the type is,** for a number, a `bool`, a
  `char` and a `String`. Another std type's, `IpAddr`'s, is an error, as its
  `parse` is.
- **The crate's is its impl's,** `{ from_str: unitFromStr_from_str }`, as
  a `Display` impl's is.
- **`T::Err` is the impl's,** shown by its dictionary's `Debug`. `String`'s
  is `Infallible`, an enum with no variants, whose `{:?}` never runs: it's
  `""`, as Rust's `match *self {}` returns nothing.

## Why

- **It's exact:** the `generic_from_str` corpus case compares `read`,
  `parse` of a `Vec` of `T`s and their errors' `Debug`, of numbers, a
  64-bit one, a `bool`, a `char`, a `String` and the crate's enum, with
  native Rust.
- **It's the JS a person writes:** a parse function given to a generic one.
