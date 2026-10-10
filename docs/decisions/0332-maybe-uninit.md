# 0332. A `MaybeUninit` is its value, or `undefined` before it's written

Status: Accepted. Extends [0074](0074-mut-boxes.md) and
[0099](0099-mut-references.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), and D for a zeroed
type no JS value is all zeros of.

## Context

A `Box` made before its value, `Box::new_uninit()` written then
`assume_init()`, and a slice of slots filled one by one, were errors:
`MaybeUninit` had no representation. `Box` was 5 of its 17 methods.
`is_std_def` asked a diagnostic item of `MaybeUninit`, a lang item that has
none, so no check of it ever matched.

## Decision

**A `MaybeUninit<T>` is what it holds, or `undefined` before it's written,
as JS has no memory that isn't a value; a `&mut` to one is a box, as a
`&mut` to a number is (ADR 0074), the place it's written to.**

```rust
let mut point = Box::<Point>::new_uninit();
point.write(Point { x: 1, y: 2 });
let zeros = unsafe { Box::<[i16]>::new_zeroed_slice(3).assume_init() };
```

```js
let point;
point = { x: 1, y: 2 };
const zeros = new Array(3).fill(0);
```

| Rust | JS |
|---|---|
| `Box::new_uninit()`, `MaybeUninit::uninit()` | `undefined` |
| `Box::write(b, v)`, `MaybeUninit::new(v)` | `v` |
| `slot.write(v)` | its place written, `point = v`, and a `&mut` to it given: the place's box for a number or text, the object for one |
| `assume_init()`, `assume_init_ref()`, `assume_init_read()` | the same value |
| `Box::new_uninit_slice(n)` | `Array.from({ length: n })` |
| `new_zeroed()`, `zeroed()`, `new_zeroed_slice(n)` | a number's `0` or `0n`, `false`, `"\0"`, a tuple or an array of them; `new Array(n).fill(0)` |
| `{:?}` of one | `MaybeUninit<u8>`, its type's name, as std's |

- **An `Option` of one is boxed**, as `undefined` before it's written
  looks like `None` (ADR 0051).
- **A `&mut` kept in a variable** of what `write` gives, `let r: &mut
  String = slot.write(..)`, names its place, `text += "!"`.
- **Refused, loud**: a zeroed type no JS value is all zero bytes of, a
  struct or a `String`; `Box`'s raw pointers, `into_raw`, `from_raw`,
  `as_ptr` and the like.

## Why

- **It's the same program**: what's written is read, as Rust reads it.
  Reading a slot before it's written is undefined in Rust; in JS it's
  `undefined`.
- **It's the JS a person writes**: a variable assigned when its value is
  ready.
- **It's tested**: the `box_uninit` corpus case runs `new_uninit` and
  `Box::write`, a struct written through a `Box` and changed through the
  `&mut` `write` gives, a slice's slots written in a loop, `new_zeroed` of
  a tuple, `new_zeroed_slice`, a `String` written and changed through a
  `&mut` kept in a variable, and an `Option` of one, against native Rust; a
  compile-fail case refuses a zeroed struct. Mutations leave a slot
  unwritten, box a `&mut` to an object, write through a handle, zero a
  `bool` as `0`, leave a zeroed slice empty, and unbox an `Option` of one.
- `docs/std-coverage.txt`: `Box` 11 of 17.
