# 0448. A `transmute` to or from a `#[repr(transparent)]` struct is it made or read

Status: Accepted; amended by [0584](0584-a-transparent-struct-is-its-field.md),
whose struct is its field, so a `transmute` is the value itself, a `&mut`'s
too. Extends [0408](0408-plain-data-is-its-bytes.md) and
[0390](0390-addressable-memory-is-refused-as-such.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)) by value and through
a `&`: the same value, read the same. Case R for a `&`'s address: the
struct made around the field is a new JS value, so `ptr::eq` of the two
references, which a program rarely asks, is `false` where Rust's is `true`.
Refused through a `&mut` (below).

## Context

A `#[repr(transparent)]` struct has its one field's layout, and Rust code
uses that to view one as the other without a copy: uuid's
`Uuid::from_bytes_ref(&[u8; 16]) -> &Uuid`, and `uuid.hyphenated()`'s
`&Uuid` as a `&Hyphenated`, each a `transmute::<&_, &_>`. rust-js read a
`transmute` of plain data, numbers alone, by its bytes (ADR 0408), but a
reference isn't plain, so each was refused as addressable memory, and uuid
with them.

## Decision

**A `transmute` between a `#[repr(transparent)]` struct and its field,
through as many such structs as wrap it, by value or through a `&`, is the
struct made of the value, as a struct literal makes it, or its field read,
as a field access reads it.**

```rust
#[repr(transparent)]
struct Uuid([u8; 16]);

fn from_bytes_ref(bytes: &[u8; 16]) -> &Uuid {
    unsafe { std::mem::transmute(bytes) }
}
```

```js
from_bytes_ref(bytes) {
  return [bytes];
}
```

1. **Made**: a tuple struct is `[value]`, a struct of a named field
   `{ name: value }`, each as `Uuid(bytes)` and `Named { name }` are.
2. **Read**: its field, `uuid[0]`, `named.name`, as `uuid.0` reads it.
3. **Through a `&`**: the same, of what it refers to; nothing writes
   through a `&`, so the new struct and the field read the same.
4. **Through a `&mut`, refused**: a write of the whole through it, `*w =
   W(..)`, would replace the new struct's field and not the caller's. A raw
   pointer too, as before.
5. Asked before plain data's bytes, so a struct of numbers is made, not
   rebuilt from its bytes: `[[9, 8, 7, 6]]`.

## Why

- **It's what Rust means**: the struct is its field, and JS has the struct
  as the value it holds.
- **It reads as a person writes it**: `[bytes]`, `uuid[0]`, the forms a
  struct's literal and field access have.
- **It's what shared code does**: uuid, and others that wrap bytes or
  strings in a newtype without copying.
- **It's tested**: a corpus case transmutes each way, through two layers,
  by value and through a `&`, of an array and a `String`, against native
  Rust; a `&mut`'s refusal is in the limitations; a mutation of each part
  is caught.

## Alternatives

- **A `&mut` too**, the new struct for it: right for writes to the field's
  parts, wrong for a write of the whole, a silent difference.
- **Plain data's bytes for a struct of numbers**: the same values, but
  rebuilt where they could be held.

## Consequences

- `ptr::eq` of a `&T` and the `&Wrapper` made from it is `false`.
- A `&mut` transmute is listed in [the limitations](../guide/limitations.md).
