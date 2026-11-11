# 0407. A raw pointer is a handle on its place

Status: Accepted. Amends [0390](0390-addressable-memory-is-refused-as-such.md)
and [0285](0285-ptr-eq.md); and by [0562](0562-a-pointer-is-a-place-and-an-address.md): a pointer's address is a number of rust-js's own, exact within its allocation; an integer is a pointer with no place.

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
    `$ptrAdd`, else rust-js's error; `offset_from`, how many items one is
    past another along one array, `$ptrOffsetFrom`, of two places rust-js's
    error, as Rust leaves it undefined (`test/corpus/pointer_offset_from.rs`);
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

## Amendment: a function pointer's address

`{:?}` of a function pointer, `main as fn()`, was refused where it was
written, so a program that compares two with `assert_eq!`, which shows
them only if they differ, didn't compile: rustc's
`layout/null-pointer-optimization.rs`. A function pointer is a pointer:
its address shown is rust-js's error where it's shown, as a raw pointer's
is, and `assert_eq!` of two compiles and runs. A runtime test shows one
and compares two.

## Amendment: a part's pointer as an array's is a view of those items (2026-11-10, #1218)

`$ptrWhole(p, n)` took only an array's first item's pointer, of an array
of `n` items, as the whole array's. A pointer into a slice's part,
`fst.as_ptr() as *const [T; N]` of rustc's `issue-70507.rs`, was one too
while a part's pointer was on the part, `$ptr(part, 0)`; since #1185
placed a part's pointer on the array it's of, `$ptr(items, at)`, that
test was refused at run time (the nightly of 2026-11-10). An item's
pointer, of `n` items from it within its array, is a pointer to those
items, `$ptrTo($view(items, at, at + n))`: read and written where they
are, a `&mut [T; N]`'s writes the array's, placed where they are as a
`&mut v[a..b]` is (ADR 0335). The first item's of the whole array is the
array's handle, as before. One past the array's end stays refused, as
reading memory by where it is. Corpus case
`pointer_to_part_of_an_array_as_an_array`, against native Rust; rustc's
`const-generics/type-dependent/issue-70507.rs` passes again.

## Amendment: a pointer's read is a copy (2026-11-11, #1265)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved.

The decision says a copy of what a pointer points to is a copy. `*p` was
one, but `p.read()` wasn't: `ptr::read`, `read_unaligned`,
`read_volatile` and `NonNull::read` gave the object in place, `p.value`.
A later write into its place through the pointer then showed in what was
read: `let d = p.add(1).read(); (*p.add(1))[0] = 9;` gave `d` as
`[9, 4]` where native Rust's is `[3, 4]`. **A pointer's read is a read of
its place, copied where `copies.rs` copies such a read
(`copy_if_needed`), as `*p` is**: `p.value.slice()` of an array whose
type is changed in place, `{ ...p.value }` of such a struct. The same
applies to `MaybeUninit`'s `assume_init_read()` (ADR 0332), which was
taken for its value itself and is now `UninitOp::Read`. It is still a
whole read for #1258's `written_whole`. `replace` and `swap` move the
value out of a place that no longer holds it, so neither needs a copy.
Corpus case `pointer_read_is_a_copy.rs`, against native Rust, both builds,
through `METAMORPHIC=all`. Mutations `pointer-read-uncopied`,
`uninit-read-uncopied` and `uninit-read-not-a-whole-read`.

## Amendment: what a pointer moves is read whole first, apart from its place (2026-11-11, #1349–#1353)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved.

The amendment above said `replace` and `swap` need no copy, as they move
a value out of a place that no longer holds it. That holds of a handle on
a slot, `$ptr(object, key)`, whose write gives the slot another value. It
doesn't hold of a handle on an object JS holds as itself, `$ptrTo`'s,
`$ptrOuter`'s, and a `$ptrWhole` view's of part of an array, whose write
is the object's in place, `$assign` (ADR 0147): what was read of it is
the object, and the write changes it. So `ptr::swap` of two `[u32; 2]`s
of one array an item apart gave `[1, 1, 1, 3]` where Rust's is
`[1, 0, 1, 3]` (#1349), and of two arrays, structs or `Vec`s that don't
overlap, each the other's (#1350); `ptr::replace` returned the new value;
`ptr::read` of a `Vec` or a struct that isn't `Copy`, followed by a write
there, read the write (#1351), the hand-written `take` and `swap` of
unsafe code; and `ptr::write(v, ..)` or `*p = ..` through a `&mut` made a
pointer, `$ptrTo(v)`, gave the name `v` another value, so the caller's
was never written (#1352).

**What a pointer moves is read whole before anything is written, apart
from its place, as std's definitions read it**:

- `$ptrSwap`, `$ptrReplace` and a one-value `$ptrCopy` read each source
  first, a handle on an object JS holds as itself giving its top level
  apart, `$apart`, as `mem::replace`'s `$take` does (`$ptrMoved`).
  No output changes.
- `ptr::read` of what isn't `Copy`, of a type a `&mut` writes whole in
  place (`MutRefForm::written_in_place`), is `$apart(p.value)`
  (`copies.rs`' `read_out`), the user's form; a `Copy` value's is
  `copy_if_needed`'s, as before. What the value holds by a pointer, a
  `Vec`'s items, a `Box`'s value, is shared, as the read shares it.
- A write through `$ptrTo(object)`, `ptr::write`'s and `*p = v`'s of a
  pointer made where it's written, is `$assign(object, v)`, the handle's
  own write: `pointee_place` marks the object it reads of `$ptrTo(object)`
  `Known::HandledObject`, and `pointers.rs`' `written_through` writes a
  place so marked in place, for `ptr::write` and MIR's `*p = v` alike.
- `ptr::copy` through a view of bytes copies each item's `width` bytes,
  `[u8; 4]`'s 4, where it copied one per item (#1356, found by the net's
  `overlaps/copy-whole`).

`ptr::copy` of more than one `[T; N]` through a `$ptrWhole` pointer, which
threw a `TypeError`, is refused where it runs, naming #1353.

Corpus cases `overlapping_moves_through_pointers.rs` (items of 1, 4 and
16 bytes, runs of 2, 3, 4 and 7, every distance from 0 to the run, each
way, of `ptr::swap`, `ptr::copy`, `copy_within`, `rotate_*`, a slice's
`swap`, `ptr::replace`, and `swap_nonoverlapping` as the control) and
`whole_values_moved_through_pointers.rs`, against native Rust in both
builds and under Miri; the memory net's `arith/swap-overlapping-array`
and `overlaps` family (ADR 0092); test/lowering-pointers.test.ts holds
the refusal.

## Amendment: a parameter taken by value whose address is taken is a cell of its own (#1381)

`fn set(mut n: u32) -> u32 { let p = &raw mut n; unsafe { *p = 7 }; n }`
threw (P0, found by m3-max's Rustlantis experiment): `n` is a local a
pointer is taken to, so it's read as its cell, `$ptr(n, "value", 0)` and
`n.value`, but a parameter is what its callers give, the number itself,
which no caller makes a cell. With a `bool`, "Cannot create property 'value'
on boolean". A parameter taken by value that `pointed().locals` names is now
a cell of the function's own, made of what it's given, as a local one is:
`function set(n) { const n$1 = { value: n }; .. return n$1.value; }`. The
parameter keeps its name, and the cell takes a fresh one (`cell_params`). A
`&mut` parameter its callers give a box of (ADR 0074) is that box, as before.
Case C of [0262](0262-when-rust-and-js-disagree.md). Held by the corpus case
`pointer_to_a_parameter_taken_by_value` (a `u32`, a `bool`, a `*const` of a
`u64`, a field of a `Copy` struct) against native Rust in both builds, and by
the mutation `pointed-param-assumed-a-cell` (`lower/mir.ts`). A write through
a pointer to a field of a struct parameter taken by value still reaches its
caller's object, filed apart as #1386, below.

## Amendment: a raw pointer to a part of a value changes that value in place (#1386)

`fn moved(mut point: Point, by: i32) -> Point { let p = &raw mut point.x;
unsafe { *p += by }; point }` changed its caller's `Copy` struct (P0, found
by audit of #1381): the pointer is `$ptr(point, "x", 0)`, a handle on the
caller's object's field, and neither side copied, because the copy policy's
census of what's changed in place (`analysis/mutation.rs`, ADR 0052) counted
a `&mut` of a part, `&mut point.x`, as a change of the value holding it, but
not `&raw mut point.x`. A raw pointer's `&raw mut` of a field, an item or a
tuple's part now counts as the `&mut`'s does, so `Point` is changed in place
and `moved(start, 10)` is given `{ ...start }`, as a `let` copying `start` is.
Unlike a `&mut`, it counts an object-like part too, `&raw mut segment.to`: a
write of a whole `Point` through it replaces the field on the segment, where
a `&mut`'s `*r = v` replaces the `Point` in place (ADR 0147). `&raw const`
isn't counted: a write through it is undefined behaviour in Rust. Case C of
[0262](0262-when-rust-and-js-disagree.md). Held by the corpus case
`raw_pointer_writes_a_copy` (a struct's field, a nested struct's, a whole
field replaced and one written through it, an array's item and one past it by
`add`, a tuple's parts, an item of an array of arrays, and copies a `let`
made) and `pointer_to_a_parameter_taken_by_value`, now printing its caller's
value too, against native Rust in both builds and through `METAMORPHIC=all`;
mutation `raw-pointer-part-write-unseen` (`lower/analysis/mutation.ts`).
