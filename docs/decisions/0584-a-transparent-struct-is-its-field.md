# 0584. A `#[repr(transparent)]` struct is its field

Status: Accepted. Amends [0448](0448-a-transparent-struct-is-its-field.md),
whose struct was `[field]` and a `transmute` made or read it. Extends
[0020](0020-structs-and-tuples.md)'s structs.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Rust lays the
struct out as its field, and nothing a program does tells them apart.
ADR 0448's Case R, `ptr::eq` of a `&T` and the `&Wrapper` made from it, is
gone: they're one value.

## Context

React's scheduler's task holds its work as a function that gives what's
left of it, of its own type: `type Callback = boolean => ?Callback`.
Rust's type alias can't be of itself, so the Rust is a newtype, its
layout its field's:

```rust
#[repr(transparent)]
pub struct Callback(Box<dyn FnMut(bool) -> Option<Callback>>);
```

A tuple struct was its array (ADR 0020), so the task's callback was
`[f]`, called `callback[0](didTimeout)`, where upstream's is the function.
ADR 0448 made a `transmute` between such a struct and its field a struct
built, `[bytes]`, or a field read; one through a `&mut` was refused, and
two `&`s of one value weren't `ptr::eq`.

## Decision

**A `#[repr(transparent)]` struct of the crate's or a dependency's is its
field in JS**, as a `ManuallyDrop` is (`recognition::transparent_field`,
the one place that answers it): made, `Callback(f)`, it's `f`; read,
`callback.0`, it's `callback`; its other fields, of no size, a
`PhantomData`, are nothing. Whether a `&mut` to it is a cell is its
field's: a `&mut Uuid` of an array is the array.

**A `transmute` between it and its field is the value itself**, by value
or through either reference, so a `&mut`'s is no longer refused.

```js
const task = { id, callback, priorityLevel, .. };
const continuationCallback = callback(didUserCallbackTimeout);
```

A struct of std's that's `#[repr(transparent)]`, a `Cell` or a
`Wrapping`, keeps the JS value rust-js has for it.

## Why

- **It's what the attribute says**: the struct is laid out as its field,
  and in JS that's being it. A newtype that wants its own shape doesn't
  ask for this layout.
- **It reads as written**: upstream's is the function; a wrapper array no
  one wrote is noise at every call.
- **One answer**: whether a struct is its field is `transparent_field`'s,
  which making, reading, projecting, a constant's value and a `transmute`
  each ask.

## Alternatives

- **Every one-field tuple struct its field**: a newtype without the
  attribute is a type of its own in Rust's eyes too, and its JS, an array,
  is public, its declarations' `[T]`. Rejected.
- **A `#[rust_js::transparent]` attribute**: Rust's attribute already says
  it. Rejected.

## Consequences

- Its TypeScript declaration is its field's type.
- A `#[repr(transparent)]` struct's `impl`s take the field's JS value as
  `self`: uuid's `Uuid` is its array.

## What holds it

- The corpus's `transparent_transmute`: each `transmute` of ADR 0448's,
  a `&mut`'s written through, `ptr::eq`, a closure's newtype called, a
  struct beside a `PhantomData`, against native Rust, by value and as a
  library.
- Mutations: a transparent struct an array again, its marker taken as its
  field, a `&mut` to it a cell, a constant's an array, a `transmute`
  refused.
