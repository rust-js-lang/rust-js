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

## Amendment: `Infallible` shown by `Display` too (#1100)

rand_core 0.9's blanket `impl<R: RngCore + ?Sized> TryRngCore for R` gives
`type Error = Infallible`, which its trait bounds `Debug + Display`. Its
dictionary asks `Infallible`'s `Display`, which was refused, "`{}` of a
`core::convert::Infallible`", so rand_core, and rand and every crate that
uses it, couldn't be compiled. `Debug` had the rule already: no value of an
enum with no variants exists, so what shows one never runs, as Rust's
`match *self {}` doesn't, and its JS is `""`. `Display` asks the same rule
now, one predicate both ask, `never_shown`, not a second copy of it.
Held by `test/corpus/infallible_shown_through_a_bound.rs`, through such a
bound and through a generic `E: Display`, against native Rust in both
builds; and by rand_core's own `fill_via_u32_chunks` and
`fill_via_u64_chunks`, run as JS by a library's `#[test]`s, as native.
Mutations `never-shown-displayed` (`lower/display/written.ts`) and
`uninhabited-debug-refused`, refitted.
