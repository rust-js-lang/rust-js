# 0407. A raw pointer is a handle on its place

Status: Accepted. Amends [0390](0390-addressable-memory-is-refused-as-such.md)
and [0285](0285-ptr-eq.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for a pointer used as
a reference is, read, written, compared, stepped along an array; D for
what reads memory by where it is, an address, a pointer read as another
type's, or a place JS may hold a copy of: a compile error, or, for an
address shown, rust-js's error where it's shown; A for a pointer only
read through, a cell of a value JS names no place of.

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

### Items of an array, as arrayvec and smallvec reach them

A vector of a fixed capacity, arrayvec's, holds a `[MaybeUninit<T>; CAP]`
and reaches its items by raw pointers (`test/corpus/array_vec_raw.rs`):

- **A cast where JS has the same place is the same handle**: a pointer to
  a `MaybeUninit<T>` and one to a `T` (ADR 0332), arrays of either too. An
  array's pointer as its first item's is `$ptr(items, 0)`; the first
  item's as the whole array's, `$ptrWhole(p, n)`, checked as it's made. In
  generic code, where it depends on the type, arrayvec's `ptr.cast::<u8>()`
  of a type with no size, `$ptrRecast(p)`, rust-js's error if it's made.
- **`ptr::copy(src, dst, n)`, `copy_nonoverlapping`, `copy_to` and
  `copy_from`** are `$ptrCopy(src, dst, n)`, of one array its
  `copyWithin`; a `Copy` value JS holds as an object something changes is
  copied into each place, as Rust's copy is, `(item) => ({ ...item })`.
- **`slice::from_raw_parts(p, n)`** is those items, a copy as a shared
  subslice is (ADR 0063), or the whole array itself; `from_raw_parts_mut`'s
  a view of them (ADR 0335).
- **`ptr::drop_in_place(p)`** runs what dropping what it points to runs,
  there, in Rust's order (ADR 0098); a raw pointer's `write`, `read` and
  `replace` keep or give back what they're given, so a generic `push` that
  writes its `T` through one takes a destructor of it (ADR 0190).
- **A constant of the crate's rustc can't say as a value**, one of a type
  parameter or a const generic, `[const { MaybeUninit::uninit() }; CAP]`,
  or a trait's `Self::CAPACITY` of an `impl<T, const CAP: usize>`, which
  resolves to its impl's now: its MIR, given its use's arguments, as code.

### A pointer to a slice

**A `*const [T]`, `*mut [T]` or `NonNull<[T]>` is the slice itself**, as
a pointer to a JS object is the object (ADR 0285): an array, or a view of
part of one (ADR 0335), which writes through to it
(`test/corpus/slice_pointers.rs`). Read through, `*p`, `as_ref()` and
`as_mut()`, it's the slice; its `len()` the slice's `length`; cast to its
item's, its first item's handle, `$ptr(items, 0)`; and
`ptr::slice_from_raw_parts(p, n)`, `slice_from_raw_parts_mut` and
`NonNull::slice_from_raw_parts` a view of `n` items from an item's
pointer, as `from_raw_parts_mut` is; `ptr::drop_in_place(tail)` drops each
of its items, arrayvec's `truncate`. arrayvec's `Drain` keeps the slice it
drains so, and hashbrown's allocator gives a `NonNull<[u8]>`. What a
pointer points to, the slice it is or the place it's a handle on, is
`pointed_place`'s alone.

A `&mut` slice's `as_mut_ptr()` is a handle on its first item, which
was refused but of a whole array: a view writes through, so its items'
places are the array's.

**A pointer only read through**, of what a reference or a box refers to,
arrayvec's `ptr::read(elt as *const _)` of a slice's item, is the place's
handle where JS names one, or else a cell of the value, `{ value }`, as a
constant's is. Only read through means each pointer made of `*r` is
dereferenced or given to `ptr::read`, or copied whole to a local that
is: never compared, kept, returned or written through, so where it is
matters to nothing, and Rust's aliasing rules let nothing change what it
points to till its last read. The program can't tell (case A). A pointer
compared, `ptr::eq(a, b)` of numbers, is still refused.

**Text's `as_ptr()`** is a pointer to a copy of its UTF-8 bytes,
`$ptr(Array.from(new TextEncoder().encode(s)), 0)`, where it's only read
through, as what a `&str` refers to may only be: given to `ptr::read`, or
what `ptr::copy` copies from, arrayvec's `ArrayString::try_push_str`
(`test/corpus/text_bytes_pointer.rs`). Compared or kept, it would be
another copy's place than Rust's one: refused, `check_text_pointer`.

### Refused

- **An address**: `p as usize`, `3 as *const T`, a pointer read as
  another type's, `{:?}` and `{:p}` of a pointer (but `0x0` of `null`),
  rust-js's error where it's shown, as only an assertion that fails shows
  one.
- **A place JS may hold a copy of**: an item of a slice, a subslice's
  reference's, a named constant's; what a `&mut` refers to through its box
  or handle, and what a reference holds, a value, where it has no place,
  but a pointer only read through. Each would be a pointer to another place than Rust's.
- **A pointer to a `str` or a `dyn T`**, which holds a length of bytes JS
  hasn't, or a vtable.

## Why

- **It's the program Rust runs**: the corpus's `raw_pointers.rs` takes
  pointers to locals, fields, objects, items, statics and literals, reads
  and writes through them, compares them alone and in `Option`s, walks a
  list linked by them, reads a generic one through `Deref`, swaps,
  replaces and steps them, boxes and unboxes, and compares Rust values with
  `ptr::eq`, each with native Rust. Twenty-eight of rustc's memory tests
  pass, none wrong; the diagnostics test refuses each kind of address and
  copy.
