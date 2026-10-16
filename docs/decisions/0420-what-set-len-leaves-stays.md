# 0420. What `Vec::set_len` leaves past a vector's length stays where it was

Status: Accepted. Extends [0417](0417-an-allocation-is-an-array-of-slots.md),
whose allocation is an array of slots, and
[0407](0407-a-raw-pointer-is-a-handle-on-its-place.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what's read past a
vector's length where `set_len` left it; D for a mutable slice over it,
rust-js's error where it's made; B for what's read past a length that
`pop` or `truncate` shortened, listed below.

## Context

`v.set_len(n)` says a vector is `n` long and does nothing else: what was
past `n` isn't dropped, but stays in the vector's memory, and a slot past
its length that's been written is in it once it's lengthened over. smallvec's
`from_vec` sets a vector's length to `0`, so it drops none of its items,
and then moves them out through its pointer:

```rust
let len = vec.len();
vec.set_len(0);
ptr::copy_nonoverlapping(vec.as_ptr(), data.inline_mut().as_ptr(), len);
```

A `Vec` is a JS array (ADR 0417), whose length is the vector's: an array
shortened has nothing past it, and the copy would read nothing. It was
refused.

## Decision

**`v.set_len(n)` is `$setLen(v, n)`: what it shortens the array by is kept
as its spare slots, `$spares`, read through a pointer past its length, and
what it lengthens it over is what was written there, or what was left.**

```js
$setLen(vec, 0);
$ptrCopy($ptr(vec, 0), $ptr(out, 0), len);
```

| Through | Past the array's length |
|---|---|
| a pointer, `*p`, `ptr::read(p)` | the slot `set_len` left there, `$slot` |
| `ptr::copy`, `copy_nonoverlapping` | the slots left there, `$slots` |
| `slice::from_raw_parts` | those, a copy, as a shared subslice is (ADR 0063) |
| `slice::from_raw_parts_mut` | rust-js's error where it's made |

- **What's at a slot is `$slot`'s**, which every read through a pointer
  asks: the array's item, or past its length, what `set_len` left there.
- **`Vec::from_raw_parts(p, len, cap)` is `$setLen` too**: its slots past
  `len` are the vector's spare memory, as `set_len`'s are.
- **A mutable slice over what's left is refused where it's made**: it'd be
  a view of the array (ADR 0335), which reads nothing past its length.
- **`set_len` keeps a vector's capacity** where it's kept (ADR 0419): it
  shortens it, `recognition::shortens_vec` says.

## Why

- **It's the program Rust runs**: `test/corpus/vec_set_len.rs` moves a
  vector's items out after `set_len(0)`, writes past a length and lengthens
  over it, reads past one through a pointer and a slice, moves items down
  over what's left as a drain does, and shortens and lengthens back, and
  prints what native Rust does; `test/lowering.test.ts` makes a mutable
  slice over what's left an error.
- **It costs only who calls it**: a vector `set_len` never shortened has
  no spare slots, and a read through a pointer is the array's.

## Consequences

- **What `pop`, `truncate` and the rest shorten a vector by isn't kept**:
  a pointer past its length reads what `set_len` left there, or nothing,
  not the item they moved or dropped. Reading a moved item again is a
  double use Rust's safety rules forbid of all but `Copy` items, which
  native Rust would read: rare and visible only to unsafe code (case B).
- A spare slot is held until the vector is collected, as Rust's memory
  holds its bytes until it's freed.
