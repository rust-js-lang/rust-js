# 0189. `panic!("{}", x)`, a slice into an array, and `into_iter()` of the crate's iterator

Status: Accepted. Extends [0012](0012-panics-and-runtime-helpers.md), [0036](0036-iterators-and-sorting.md)
and [0055](0055-iterator.md).

## Context

chrono stopped at three of std's functions besides `Duration`'s (ADR 0188):

- **`panic!("{}", msg)`,** which std writes as `panic_display(&msg)`, not
  as the `panic_fmt` of a `format_args!` another panic is.
- **`<&[u8; 19]>::try_from(&bytes[..19])`,** a slice into an array of its
  length, as it reads a date's digits.
- **`self.into_iter()` of its own `StrftimeItems`,** an iterator, whose
  `into_iter` is std's for any iterator.

## Decision

- **`panic!("{}", x)` is a panic of `x`'s `Display`,** `throw new
  Error(msg)`, as `panic!("{x}")` is.
- **A slice into an array is `Ok` of the slice where it's the array's
  length, else `Err` of a `TryFromSliceError`:**

  ```js
  slice.length === 3 ? { TAG: "Ok", _0: slice } : { TAG: "Err", _0: undefined }
  ```

  The slice is a copy already (ADR 0036), so an array by value, of a
  `Copy` item, is it too. One into a `&mut` array, which writes the
  slice's items, is an error. A `TryFromSliceError` shows as std's,
  `TryFromSliceError(())` and `could not convert slice to array`.
- **`into_iter()` of an iterator of the crate's is the iterator itself,**
  as std's is for any iterator, and as an array's and a type parameter's
  are already.

## Why

- **Each is exact:** the `panic_display`, `slice_to_array` and
  `user_into_iter` corpus cases compare each, with an array by value and
  by reference, one of the wrong length, and its error's `{:?}` and `{}`,
  with native Rust.

## Costs

- **A slice into a `&mut` array is still an error.**
