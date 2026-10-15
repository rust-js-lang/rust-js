# 0407. A raw pointer is a handle on its place

Status: Accepted. Amends [0390](0390-addressable-memory-is-refused-as-such.md)
and [0285](0285-ptr-eq.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for a pointer used as
a reference is, read, written, compared, stepped along an array; D for
what reads memory by where it is, an address, a pointer read as another
type's, or a place JS may hold a copy of: a compile error, or, for an
address shown, rust-js's error where it's shown.

## Context

ADR 0390 refused raw pointers as addressable memory. But most code that
uses one uses it as a reference: `&mut x as *mut i32`, `*p = 5`,
`p.is_null()`, `p == q`, a list linked by `*mut Node`, `ptr::swap`.
About seventy of rustc's tests refused do only that.

## Decision

**A raw pointer is a handle on its place, `{ value }`, one for each
place, so two pointers to one place are `===`; or `null`.**

```js
const x = { value: 123 };
const p = $ptr(x, "value");
p.value = 5;
const node = { value: 1, next: $ptrTo(second) };
```

- **What it's a handle on**:
  - an object, by the object, `$ptrTo(obj)`, whose `*p = v` makes it
    `v` in place;
  - a field or an item, `$ptr(object, key)`, cached per place;
  - a local a pointer is taken to the whole of, `&raw const x` or
    `&x as *const _`, is a cell, `{ value }`, and its pointer
    `$ptr(x, "value")`;
  - a constant's, `&3`, a cell of its own;
  - a `Box::into_raw`'s, a place of its own.
- **`*p` is `p.value`**, written `object[key]` where the pointer is
  made there; `p == q`, `ptr::eq`, and what holds one comparing it, by
  identity.
- **A copy of what a pointer points to is a copy**: the mutation
  analysis counts the type as changed, so no two places share one object,
  and `ptr::eq` of Rust values, which ADR 0285 refused, is the places'.
- **std's functions**:
  - `null`, `is_null`, `read`, `write`, `replace`, `swap`;
  - `add`, `offset` and `sub`, stepping an array's item to another's,
    `$ptrAdd`, else rust-js's error;
  - `as_ref`, `as_mut`, and `cast` to the same type or `()`;
  - a slice's or a `Vec`'s `as_ptr`, `Box::into_raw` and `from_raw`.
- **A pointer nothing reads** is made of nothing; one to an immutable
  static is a handle on it, or a cell of its value, which nothing writes.
- **A `ref mut` binding read back out through a match guard** is a value
  of its own, its handle, which it wasn't, and read `n.value` of a number
  (`borrowed`).

### A `NonNull`

**A `NonNull<T>` is its raw pointer**, never `null`, as a `Pin` is its
pointer (ADR 0329): a handle on its place. `NonNull::new(p)` and
`new_unchecked(p)` are `p`, an `Option` of one being `None` of `null`, as
JS reads it `!= null` (ADR 0030); `as_ptr()` is the pointer; `as_ref()`
and `as_mut()` are its place, never `None`; `cast`, `add`, `sub`, `read`,
`write`, `replace` and `swap` are a raw pointer's; `NonNull::from(&mut x)`
is a pointer to `x`, `$ptrTo` of an object, and the `&mut`'s handle of
what isn't one. Whether a type is a raw pointer, a `NonNull` too, is
`pointers::raw_pointee`'s alone, which the architecture test holds.
`test/corpus/non_null.rs` holds it, a list of nodes linked by `NonNull`
too. Found by the crates.io corpus: anyhow, smallvec, semver and
hashbrown's types hold one, where 912 of 1,398 refusals were of it.

### Refused

- **An address**: `p as usize`, `3 as *const T`, a pointer read as
  another type's, `{:?}` and `{:p}` of a pointer (but `0x0` of `null`),
  rust-js's error where it's shown, as only an assertion that fails shows
  one.
- **A place JS may hold a copy of**: an item of a slice, a slice's
  `as_ptr` but of a whole array made one just there, a subslice's
  reference's, a named constant's; what a `&mut` refers to through its box
  or handle; what a shared reference holds, a value, where it has no
  place. Each would be a pointer to another place than Rust's.
- **A pointer to what has no size**, `*const [T]`, `*const dyn T`, which
  holds a length or a vtable too.

## Why

- **It's the program Rust runs**: the corpus's `raw_pointers.rs` takes
  pointers to locals, fields, objects, items, statics and literals, reads
  and writes through them, compares them alone and in `Option`s, walks a
  list linked by them, reads a generic one through `Deref`, swaps,
  replaces and steps them, boxes and unboxes, and compares Rust values with
  `ptr::eq`, each with native Rust. Twenty-eight of rustc's memory tests
  pass, none wrong; the diagnostics test refuses each kind of address and
  copy.
