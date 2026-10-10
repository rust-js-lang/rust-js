# 0338. A slice's `split_off`, and a `&mut` to a shared reference

Status: Accepted. Extends [0099](0099-mut-references.md) and
[0335](0335-slice-views.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`split_off`, `split_off_first` and `split_off_last`, and their `_mut`
ones, take a `&mut` to a slice reference, `self: &mut &'a [T]`, and
change it to what's left after what they take. A `&mut` to a shared
reference, `&mut &str` or `&mut &[T]`, wasn't a value rust-js had: a
reference was never boxable, so `fn shorten(s: &mut &str) { *s = &s[1..] }`
was an error. `slice` was 107 of 133.

## Decision

**A `&mut` to a shared reference is a cell of it, as one to a number is
(ADR 0099): the reference is a value JS can't change in place, only
replace. A slice's `split_off` gives its place what's left and gives back
what it takes, as a `String`'s `pop` does (ADR 0149).**

```rust
fn shorten(s: &mut &str) {
    *s = &s[1..];
}
let word = input.split_off(..end)?;
```

```js
function shorten(s) {
  s.value = $strSlice(s.value, 1);
}
const split = $sliceSplitOff(input.value, end);
input.value = split[0];
const word = split[1];
```

| Rust | JS |
|---|---|
| `s.split_off(..n)`, `split_off(n..)` | `$sliceSplitOff(s, n)`, `$sliceSplitOff(s, n, true)`: what's left and what's taken, or `s` and `undefined` past its end |
| `split_off_first()`, `split_off_last()` | `$sliceSplitOffEnd(s, last)`: what's left and the item, or `s` and `undefined` of an empty one |
| their `_mut` ones | `$sliceSplitOffMut`, `$sliceSplitOffEndMut`: views (ADR 0335), the item a handle on a number or text |
| `&mut s` of a `&str` or a `&[T]` | a box of it given and taken back, or a handle, as of a number |

- **A `&mut` reference stays what it was**: a `&mut &mut T` is still not a
  cell; only a shared one is, which nothing writes through.
- **Since**: a map's `retain` of `&str` values gives the closure handles on
  them, so `*v = ".."` in it is kept.

## Why

- **It's the same program**: each write through such a `&mut` replaces
  the reference it came from, and each `split_off` leaves what Rust's
  leaves.
- **It's the JS a person writes**: `s = split[0]`, a variable given its
  new value.
- **It's tested**: the `slice_split_off` corpus case runs, against native
  Rust, each form of a shared slice, past the end, of an empty slice, a
  tokenizer of `&mut &[u8]`, and the `_mut` ones of numbers and strings;
  `mut_ref_to_ref` replaces a `&mut &str` given to a function twice, a
  `&mut &[i32]` by a longer one, `&str` items through `iter_mut`, one in
  a variable, and a field's. Mutations keep references unboxable, leave
  the place unwritten, take `split_off(n..)` for `..n`, drop the handle,
  copy for a view, refuse it, and let the empty and past-the-end cases
  through.
- `docs/std-coverage.txt`: `slice` 113 of 133.
