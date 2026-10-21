# 0408. Plain data is its bytes

Status: Accepted. Amends [0390](0390-addressable-memory-is-refused-as-such.md)
and [0332](0332-maybe-uninit.md); and by [0562](0562-a-pointer-is-a-place-and-an-address.md): a thin pointer, and a reference to an object, is a 4-byte scalar, its address's.

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for a `transmute` of
plain data, which reads the same bytes rustc's would, and a `zeroed` one;
D for one of anything else, whose bytes are an address or a JS value's: a
compile error, as before.

## Context

ADR 0390 refused `transmute` and `mem::zeroed` as addressable memory. But
most programs that use them never take an address: they read a float's
bits, an integer's bytes, a `#[repr(C)]` struct as an array, a packed
struct's layout, or make a value all zero, `mem::zeroed()`. Those are
bytes of numbers, which JS has: a `DataView` writes and reads them, and
rust-js already gives `to_le_bytes`, `from_le_bytes`, `to_bits` and
`from_bits` by them (ADR 0156). rustc's `transmute` and `zeroed` tests,
and its packed-layout ones, stopped there.

## Decision

**Plain data is a number, a `bool`, a `char`, or a tuple, an array or a
struct of the crate's or a dependency's of them. Its layout is rustc's
for wasm32 (ADR 0090), each scalar at its offset. `transmute::<A, B>` of
plain data makes a `B` of `A`'s bytes: each of `B`'s scalars is the one of
`A` at the same offset, of the same size, retyped; else it's read from the
bytes of the whole, little-endian, as wasm32's are. `mem::zeroed()` makes
each scalar zero.**

```rust
let bits: u32 = mem::transmute(1.0f32);
let bytes: [u8; 4] = mem::transmute(0x1234_5678u32);
let halves: Halves = mem::transmute(0xdead_beefu32);
let point: Point = mem::zeroed();
```

```js
const bits = $floatToBits(1, 4);
const bytes = $toBytes(305419896, 4, true);
const bytes$1 = $toBytes(3735928559, 4, true);
const halves = {
  low: $fromBytes(bytes$1.slice(0, 2), 2, true, false),
  high: $fromBytes(bytes$1.slice(2, 4), 2, true, false),
};
const point = { x: 0, y: 0n, on: false };
```

- **A scalar retyped is through its bits**, an unsigned integer of its
  size: an integer's `as`, a float's `$floatToBits` and `$floatFromBits`,
  a `bool`'s `1`/`0` and `!== 0`, a `char`'s code point and
  `String.fromCodePoint`.
- **The bytes of the whole** are each scalar's `$toBytes`, zeros for the
  padding between, made once; a `[u8; N]`'s are itself, and one scalar's
  alone its `$toBytes`. A `[u8; N]` made of them is them. A scalar is read from them by `$fromBytes`, signed
  as its type is.
- **A type that differs only in its lifetimes** is the same value:
  `transmute::<&'a str, &'static str>(s)` is `s`.
- **`mem::zeroed()` is `MaybeUninit::zeroed()`'s zero** (ADR 0332): one
  answer for both, `plain.rs`'s, and now of a struct too.
- **Only Rust's own scalars**: std's structs JS has as a value of its own,
  a `Duration`'s nanoseconds, a `Wrapping`'s number, aren't their bytes.
- **The layout is one question**, `FnCx::layout`, which `size_of`,
  `align_of`, a sort's item size and plain data's offsets all ask.

## Why

- **It's the bytes Rust reads**: the `plain_data` corpus case compares
  each with native Rust: a part retyped, signed and not, floats, `bool`,
  `char`, unit, a 128-bit integer, a `#[repr(C)]` struct to and from an
  integer and an array, across a struct's padding and an aligned field,
  and zeroed numbers, tuples, arrays and structs.
- **It's JS's own**: a `DataView`'s bytes, through the helpers that
  already read them.
- Seven of rustc's tests pass: `transmute-array-to-scalar.rs`,
  `issue-20343.rs`, `for-loop-has-unit-body.rs`, `struct-offsets.rs`, and
  the three `packed` layout tests.

## Consequences

- **Still refused**, as addressable memory: a `transmute` of a reference, a
  pointer, a `Box`, a function, an enum or a union, whose bytes are an
  address or a tag JS's values don't have, and of a type parameter, laid
  out by its caller; a zeroed one of those.
- **A union's field read as another's** is still rust-js's error where it
  runs (ADR 0406), though plain data's bytes could answer it.
- **Padding is never read**: its bytes are undefined behavior's to read, so
  what the JS gives for them is whatever the zeros make.
- `transmute_copy` and `mem::uninitialized` are still refused.
