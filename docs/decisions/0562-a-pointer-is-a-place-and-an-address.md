# 0562. A pointer is a place and an address; bytes are plain data's

Status: Accepted, amended by [1114](1114-a-reference-a-pointer-may-leave-with-is-its-places-pointer.md) (2026-11-10: a raw pointer of a type parameter is a handle in generic code, adapted at the boundary), delivered in stages: each stage below names the tests
that hold it, and a stage not yet landed is listed under Consequences as
still refused. Amends [0390](0390-addressable-memory-is-refused-as-such.md),
[0407](0407-a-raw-pointer-is-a-handle-on-its-place.md),
[0408](0408-plain-data-is-its-bytes.md) and
[0406](0406-a-union-is-the-field-it-holds.md); extends
[0417](0417-an-allocation-is-an-array-of-slots.md) and
[0285](0285-ptr-eq.md). Replaces the draft `bytes-through-a-pointer`.
Extended by
[0864](0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md): a hash of
an address.

Case ([0262](0262-when-rust-and-js-disagree.md)): C for what a pointer
reads, writes, compares and measures within one allocation, and for the
bytes of plain data; A for an address's value, which Rust's own program
can't rely on either; B for an address shown; D for a pointer to a place
JS has no name for, and for bytes of what isn't plain data.

## Context

JavaScript has no memory a program can address. ADR 0390 refused
everything that does, and ADR 0407 took the first step back: a raw
pointer is a handle on its place, `{ value }`, one per place, so a pointer
used as a reference is, which was about seventy of rustc's tests. What's
left is what a handle can't say. Of rustc's 99 known failures, 68 are
memory, in these shapes:

| Shape | Tests | What it needs |
|---|---|---|
| a `transmute` of what isn't numbers alone | 16 | pointers and references as plain data; a byte slice as text |
| a pointer to a place JS doesn't name | 10 | a number reached through a `&`: stays refused |
| a static or a constant holding a pointer | 9 | a pointer rustc's const-eval made, to a static or an interned value |
| an integer as a pointer, a pointer as an integer, or shown | 12 | an address |
| bytes read as another type's, a union's field | 9 | bytes of plain data |
| a `*const dyn Trait` | 3 | a fat pointer as the `dyn`'s pair |
| a pointer into a copy, `NonNull::dangling` | 4 | a view (ADR 0063's copies), an address with no place |

And 6 of the 22 crates in `docs/crate-corpus.md` stop here. What each
does is narrower than "memory":

- **aho-corasick, so regex**, and memchr's `memmem`: read a `u16` or a
  `u32` from a `*const u8` into a byte slice, `read_unaligned`, compare
  and subtract pointers into one slice, and take `p as usize` for
  offsets.
- **semver**: packs a heap pointer with a tag, `(p as usize | 1)
  .rotate_right(1)`, into a `NonNull<u8>` it later unpacks, and stores
  short identifiers *in the pointer's own bytes*, read back as a `u64`
  through `addr_of!(*repr).cast::<NonZeroU64>()`.
- **once_cell**: a queue of waiters linked by `*mut Waiter` with the
  state in its low two bits, `addr(q) & 3`, `q | RUNNING`, masked off
  before any read.
- **anyhow**: a `#[repr(C)]` `ErrorImpl<E>` reached through a pointer to
  `ErrorImpl<()>`, its prefix, and the vtable read by casting the struct's
  pointer to its first field's type, `*(p as *const &'static ErrorVTable)`.
- **yoke, so url**: a pointer made of a sentinel integer, and casts
  between a type and itself with lifetimes erased.
- **powerfmt, so time**: a `[MaybeUninit<u8>; N]` read back as a `[u8]`.

Every one of these is exact on wasm32, the target rust-js checks programs
for (ADR 0090): a byte is a `u8`, integers are little-endian, every type
has the size, alignment and field offsets rustc's layout gives, and a
`usize` is 32 bits. What Rust itself leaves to the allocator is only
*where* an allocation is, and no Rust program can rely on that: addresses
differ from one run to the next, and comparing pointers into different
allocations is unspecified. Rust's own model of this, strict provenance,
says a pointer is a *provenance*, which allocation it may touch, and an
*address*, a number; an integer becomes a usable pointer only if it was
made from a pointer that exposed its address. That is a model JS can
keep, and it's the one this decision takes.

## Decision

**A pointer is a place and an address. Its place is ADR 0407's handle,
or none. Its address is a 32-bit number, exact within its allocation:
the allocation's base, given once and never shared, plus the byte offset
rustc's layout gives the place. Bytes are plain data's (ADR 0408), read
and written through a pointer by that layout. What has neither a place
JS names nor bytes that are numbers' is refused, as before.**

```rust
let p = v.as_ptr();                              // [u8]
let word = unsafe { p.add(4).cast::<u32>().read_unaligned() };
let tagged = (p as usize | 1) as *const u8;
let back = (tagged as usize & !1) as *const u8;  // == p
```

```js
const p = $ptr(v, 0, 0);
const word = $readInt($ptrAdd(p, 4, 1), 4, false);
const tagged = $ptrFrom(($addr(p, 1) | 1) >>> 0, 1);
const back = $ptrFrom(($addr(tagged, 1) & ~1) >>> 0, 1);
```

### A pointer's address

- **A handle knows its byte offset in its allocation.** A field's is
  written where the pointer is made, `$ptr(record, "b", 4)`, rustc's
  layout of the struct for wasm32, where that's known here; an item's is
  its index times its size, which whoever asks the address gives,
  `$addr(p, 4)`, so `$ptr(items, i)` and `$ptrAdd(p, n)` stay as they
  were, and generic code that never asks an address never carries a size.
  `$ptrTo(object)`'s and a cell's are 0. An allocation is the JS value a
  handle is on: a struct held in another, or in an array, is its own, so
  its fields' addresses relate to each other, not to its holder's.
- **`add`, `offset` and `sub` step by items**, as before, which Rust
  keeps inside the allocation; **`wrapping_add`, `wrapping_offset`,
  `wrapping_sub`, `byte_add` and `wrapping_byte_add` move the address**,
  `$ptrAt(p, $addr(p, size) + n * size, size)`, which may leave the
  allocation and come back, as semver's does.
- **An allocation's base is given when an address is first asked of it**,
  `$addr(p, size)`: the next free multiple of 4096 after the last
  allocation, which reserves its own length in bytes, rounded up to 4096,
  and never gives it back. So two live allocations never overlap, every
  base is aligned past any type's alignment, and addresses rise in the
  order they're asked. An allocation is what ADR 0417 says it is: a JS
  array, an object, or a cell. The address space is 32-bit, as wasm32's;
  a program that exhausts it meets rust-js's error, as it would meet
  Rust's out of memory.
- **`p as usize`, `p.addr()`, `p.expose_provenance()` and a `transmute` of
  a pointer to an integer** are `$addr(p)`, which also *exposes* the
  allocation: records its base and extent. `usize as *const T`,
  `with_exposed_provenance(n)` and the reverse `transmute` are
  `$ptrFrom(n, size)`: the handle at `n` in an exposed allocation, found by
  base and offset, `$ptr(of, (n - base) / size, n - base)` of an array, the
  object or the cell at offset 0, or a field handle that itself exposed
  `n`; of no exposed allocation, **a pointer with no place**, whose
  address is `n` and whose `value` is rust-js's error. That is strict
  provenance exactly: `1 as *const T` is a pointer with address 1, which
  `as usize` gives back and `*p` refuses.
- **A pointer with a place and another address**, `with_addr`,
  `map_addr`, `wrapping_add` and `wrapping_byte_add` past its place,
  once_cell's `q | RUNNING`, is `$ptrAt(p, n)`: the place's handle where
  `n` is its own address again, else a pointer with no place whose
  provenance is `p`'s, so `$ptrAt(tagged, addr & !3)` is the waiter's
  handle, and reading through the tagged one is the error it is in Rust.
- **`p == q`, `<`, `offset_from` and a pointer's hash are its address's.**
  `$ptrEq(p, q)` is the same place, the same array and index or object
  and field, or the same address, `null`'s 0: so `1 as *const T == 1 as
  *const T`, and a handle's identity, which ADR 0407 compared by, no
  longer matters; a handle is still made once per place, as `ptr::eq`
  of two references compares the objects they are. `$ptrOffsetFrom` of two
  allocations stays rust-js's error, as Rust leaves it undefined.
- **A pointer to a zero-sized type, and `NonNull::dangling()`**, is a
  pointer with no place whose address is the type's alignment, as Rust's
  is; `wrapping_byte_add` on it is arithmetic.
- **`{:p}` and `{:?}` of a pointer show its address**, `0x` and hex, as
  Rust shows one. The number is rust-js's, not an operating system's: case
  B, as Rust's own differs each run, and the same program printing the
  same address twice prints the same number.
- **A pointer is plain data of 4 bytes**: its bytes are its address's,
  so a struct holding one is laid out and `transmute`d as rustc lays it
  out, and bytes `transmute`d into a pointer are `$ptrFrom` of them.
  semver's inline identifier, eight ASCII bytes in a `NonNull<u8>` and a
  `[u8; 4]`, is exactly that.

### Bytes through a pointer

- **A pointer into an array of integers is a pointer to its bytes**,
  `$ptrBytes(p, stride)`, where a cast makes it one, `p.cast::<u8>()` of a
  `*const u32`, or `p.cast::<u32>()` of a `*const u8`: a view of the
  array as the bytes wasm32 holds, little-endian, each item `stride` of
  them. `read`, `read_unaligned`, `write` and `write_unaligned` of a
  `width`-byte integer at byte offset `o` are `$readInt(view, width,
  signed)` and `$writeInt`, which read or patch the items the bytes fall
  in, O(1) and no copy; `read` of a `[u8; N]` is `$slots` of them;
  `copy`, `from_raw_parts` and `offset_from` count bytes. Of a `[u8]`
  array, which is its own bytes, the view is the array itself.
- **A pointer into plain data is a pointer to its bytes** the same way:
  a struct or tuple of numbers (ADR 0408), a pointer among them, whose
  bytes are made by ADR 0408's codec where read and decoded back where
  written, `$ptrBytes(p, encode, decode)`. Reading the bytes of what
  isn't plain data, a `String`'s or a `Vec`'s, stays refused where the
  cast is made.
- **A union's field read as another's is its bytes decoded**: the field
  it holds, encoded as plain data, read as the field asked, by the codec
  ADR 0408's `transmute` already writes, at the read site, for each
  other plain field. A union with a field that isn't plain data stays
  what ADR 0406 made it.
- **A cast between pointees keeps the place where rustc's layout puts
  the target there**: the same type but for lifetimes, `()`, a
  `MaybeUninit<T>` and its `T` (ADR 0332), an array and its first item,
  and now **a struct and the field rustc lays out at its offset 0**, anyhow's
  `*const ErrorImpl` as `*const &'static ErrorVTable`, and **two structs
  whose layouts share a prefix**, `#[repr(C)]` `ErrorImpl<E>` as
  `ErrorImpl<()>`: the same handle, the fields read by name. A field read
  that the target has and the object hasn't is `undefined`, which only
  undefined behaviour in the Rust reaches (case A). `pointers::recast` is
  the one owner of a cast between pointees, asked of rustc's layout where
  it's known here; where only the value can tell, a type parameter's, it
  emits a check made where the pointer is, `$ptrFirst`, `$ptrWhole`,
  `$ptrRecast`.

### Fat pointers

- **A `*const dyn Trait` is the `dyn`'s pair**, `{ value, impl }`, as a
  `Box<dyn Trait>` and an `Rc<dyn Trait>` are (ADRs 0331, 0520), and a
  `*const [T]` is the slice (ADR 0407). A thin pointer unsized to one,
  `p as *const dyn Trait`, is the pair of its place's value and the
  dictionary the cast names; a fat one made thin, `p as *const U`, is
  `$ptrTo(pair.value)` of an object. `*p` is the pair, called through its
  dictionary. anyhow's `Box<dyn Error>` taken apart and put back is this.

### Constants and statics

- **A pointer rustc's const-eval made** is made as its initializer says,
  the initializer lowered as code, as a constant rustc can't say as a
  value tree is (ADR 0127): an integer made a pointer, semver's `HEAD`,
  is its address; a null is `null`; `&42 as *const i32`, memory rustc
  interned, is a cell of the value, as a constant's place is (ADR 0407),
  and a static's field holding it is made once with the static. A
  constant pointing into a static, `&raw const COUNT`, stays refused:
  made once at the module's level it would read the static before the
  module makes it, and the static's address isn't a module's to copy
  (ADR 0096).

### What stays refused, as the one place says

`refusal.rs` words each, "rust-js does not support addressable memory
yet: ..", with why (ADR 0390):

- **A pointer to a number reached through a `&`**: `fn f(x: &u32) { &*x
  as *const u32 }`. The reference is the number itself in JS, with no
  place to be a handle on, as ADR 0285 says; one only read through is a
  cell of it still (ADR 0407).
- **A pointer into a shared subslice**, which ADR 0063 makes a copy:
  compared, kept or exposed, it's another copy's place.
- **Bytes of what isn't plain data**, and a `transmute` between such
  types: a function pointer retyped, an enum whose tag or niche rustc
  lays out, a `Box` as an integer. A `Box<[u8]>` as a `Box<str>` is text
  of the bytes, which only invalid UTF-8, undefined behaviour, could tell
  apart.
- **A pointer to a type aligned past 4096**, `#[repr(align(8192))]`,
  whose base this decision can't promise, where the type is known. In
  generic code, `is_aligned_to<T>(p: &T)`, the type isn't, and the
  address is 4096-aligned: case B, listed here; rustc's
  `align-struct.rs`, which asks a 65536-byte alignment, stays a known
  failure.
- **The address of a field of a struct its caller lays out**, `s.b` of a
  `Pair<T>`, whose offset only the caller knows: rust-js's error where
  it's asked.
- **A pointer as a map's key**, `HashSet<*const T>`, which needs the
  map's key encoding to be its address: a later stage. A struct holding
  one hashes it as its address.
- **A constant pointing into a static**, `const P: *const u32 = &raw
  const COUNT`, and a raw address of a static in a body (ADR 0096):
  "constants of type `*const u32`" and "raw addresses of statics".

## Why

- **It's Rust's own model.** Strict provenance is what rustc's documentation
  says a pointer is, and `with_exposed_provenance` is what the standard
  library says an integer-to-pointer cast means: a pointer is usable only
  through an exposed allocation. rust-js implements that model rather than
  a flat memory, so a valid program sees what wasm32 gives it, and an
  invalid one sees an error, not another program's bytes.
- **It's exact where a program can look.** Offsets, alignment, sizes,
  little-endian bytes and the order of addresses within an allocation are
  rustc's for wasm32 and wasm32's, the same numbers native Rust computes,
  held against native Rust by the corpus cases below. What differs, an
  allocation's base, is what native Rust can't promise either.
- **It's the bytes that are there.** A byte slice is already an array of
  its bytes, and plain data's bytes are already written (ADR 0408): no
  value changes its representation, and the app's JS is untouched; a
  pointer appears in it only where the Rust has one, and then as
  `$ptr(v, i, i * 4)` for what was `$ptr(v, i)`.
- **It takes the long tail at once.** Of the 68 memory failures in rustc's
  suite, 58 fall in the shapes above and only the 10 pointers to a `&` of
  a number stay; of the 6 blocked crates, 5 use nothing but these shapes.
  once_cell also waits on `thread::current()` and `park()`, which are a
  decision of their own, and regex on the per-item library refusal
  draft.
- **It's held against native Rust**, stage by stage, below.

## Alternatives

- **A linear memory**, every value at an address in an `ArrayBuffer`, as
  wasm-bindgen's output is. Exact for everything, and the opposite of the
  north star: a `Point` becomes offsets, a `Vec<String>` becomes
  pointers, no JS caller can read the output, and rust-js exists because
  that output isn't JavaScript people would write. Rejected.
- **Keep refusing, and qualify crates instead** (ADR 0529): a shipped
  memchr answers aho-corasick's reads without a pointer model. It
  answers one crate at a time, by hand, for a model every unsafe crate
  shares; semver and once_cell have nothing to qualify them against.
  Rejected as the only way; it stays the way for what reads memory JS
  can't model, as memchr's SIMD is.
- **Addresses from a JS object's identity**, hashing it: no order, no
  offsets, no alignment, nothing `% align` could be exact about.
  Rejected.
- **A base aligned only to the type's alignment**: exact for alignment
  checks too, but a field handle's allocation type isn't known where the
  field's pointer is made. 4096 is aligned past every alignment a type
  has in practice, so `addr % align` is 0 at a base and the field's
  offset at a field, both Rust's.

## Consequences

- **The app's JS is unchanged** where it has no pointers, which is nearly
  everywhere; a handle gains a number, and the helpers that read bytes or
  addresses are emitted only where the Rust asks.
- **A `Vec` that grows keeps its addresses**: Rust's would move. A
  program reading through a pointer after the growth is undefined
  behaviour in Rust, and in rust-js reads the array, which is case A.
- **A `u32` read from a byte slice is four array reads**, slower than
  the byte loop it replaces in Rust's fast paths; the memchr probe
  measures it (ADR 0529's probe ran fifteen times faster on `indexOf`,
  and a shipped memchr stays the answer where that matters).
- **The measure of std's bodies** (ADR 0561) counts `read_unaligned`, a
  pointer cast and `p as usize` as `memory`; after this decision they're
  the vocabulary where the pointee is plain data. `pointers::mir_addresses_memory`
  stays the one reading, narrowed to what's still refused, and
  `docs/std-coverage.txt` reblesses.
- **Still refused until its stage lands**, listed as each lands:
  stages 2 to 4 below, each a compile error in the words above until
  then.

## Delivery

Each stage lands with a failing corpus case first, its mutations, and
rustc's suite reblessed.

1. **An address** (landed): a field's offset in its handle, `$addr`,
   `$ptrFrom`, `$ptrAt`, `$ptrEq`, pointer `==`, `<`, hash and `{:p}`,
   `as` casts both ways, `addr`, `expose_provenance`,
   `with_exposed_provenance`, `with_addr`, `map_addr`, `wrapping_add`,
   `wrapping_byte_add`, `is_aligned`, `dangling`, a pointer as plain
   data. `pointer_addresses` and `tagged_pointers`.
2. **Bytes** (landed): `$ptrBytes` views of integer arrays and of plain
   data, `$readInt`, `$writeInt`, `read_unaligned`, `[u8; N]` reads, a
   union's bytes, `Box<[u8]>` as text. `bytes_through_pointers` and
   `union_bytes`; aho-corasick, memchr's `memmem` and powerfmt measured.
3. **Fat pointers** (landed): `*const dyn Trait` as the pair, unsized
   from a thin pointer, called and written through, made thin again, a
   `Box<dyn Trait>` taken apart and put back. `dyn_pointers`.
4. **Relayout** (landed): the first-field and prefix rules, in `recast`,
   the one owner of a cast between pointees; `$ptrFirst`, `$ptrWhole` and
   `$ptrRecast` stay its checks of what only the value can tell, a type
   parameter's. `pointer_relayout`; anyhow and semver measured.
5. **Constants and statics** holding pointers (landed): made as their
   initializers say, the one route a value rustc can't say as a value
   tree takes, now open to a type holding a pointer, which is shareable
   as a handle is, and a function's own or an associated constant rustc
   can't say as a value, semver's `HEAD` and `Prerelease::EMPTY`, is its
   MIR as code (ADR 0127). `pointer_constants`; semver compiles.

## What holds it

- `test/corpus/pointer_addresses.rs`: `p as usize` of an array's items
  differing by `size_of::<T>()`, `% align_of::<T>()` of a base and of a
  field, the stride of packed items, a pointer made of its own address
  dereferenced, `transmute` of a pointer to its address, `1 as *const T`
  compared with another, `{:p}` of one place shown the same twice,
  pointers ordered along an array, `with_addr` and `map_addr` back to a
  place, dangling pointers at their alignment, against native Rust.
- `test/corpus/tagged_pointers.rs`: once_cell's state in a waiter
  pointer's low bits, masked back to the waiter and written through;
  semver's heap pointer rotated into a representation no ASCII is, and
  back, each against native Rust. `test/runtime-package.test.ts` holds
  that reading through a pointer to no place is rust-js's error where it
  runs, and that an address is shown.
- `test/corpus/bytes_through_pointers.rs`: a byte slice read as `u32`s
  and a `u16` at any offset, as `[u8; 3]` and as an `i8`, a `u32`
  pointer stepped over it, aho-corasick's `is_equal_raw` four bytes at a
  time, a `u32` and a `u16` written into a byte buffer and read back as
  bytes and as an `i8`, `Box<[u8]>` transmuted to `Box<str>`, a `u32`
  array's bytes read across an item, sliced, patched and copied into, and
  a `#[repr(C)]` struct's bytes read, sliced and written through a byte
  pointer, each against native Rust.
- `test/corpus/union_bytes.rs`: a union's `u32` read as its `f32`, its
  `[u8; 4]` and its `(u16, u16)`, a `f32` written and read as `u32` and
  bytes, bytes written and read as `u32` and halves, and a `u64` read as
  `[u32; 2]` and `u8`, against native Rust. `test/runtime-package.test.ts`
  holds that a union's `u32` reads as its `f32`, and that a field whose
  bytes JS has none of, a `&str`, is rust-js's error where it's read.
- `test/corpus/dyn_pointers.rs`: a `*const dyn Shape` made of a `&Square`,
  called through `&*z` and `*z`, `is_null`, a `*mut dyn Shape` written
  through, `z as *const Square` read and equal to `&raw const square`, a
  `Box<dyn Shape>` taken apart with `into_raw`, written through and put
  back with `from_raw`, and a struct holding `*const dyn Shape`s called
  later, against native Rust.
- `test/corpus/pointer_relayout.rs`: an anyhow-shaped `#[repr(C)]`
  `Impl<E>` with its vtable first, read through `*(p as *const &'static
  VTable)`, erased to `Impl<()>` and unerased in generic code, its fields
  read through each, and taken back as a `Box<Impl<u32>>`, against native
  Rust.
- `test/corpus/pointer_constants.rs`: a static holding a null pointer and
  one holding a pointer to interned memory, read through, a `NonNull`
  constant of all ones compared with `usize::MAX`, a dangling constant's
  address, a null constant, a function's own constant and an associated
  constant holding a pointer, semver's `HEAD` and `Prerelease::EMPTY`,
  against native Rust; rustc's
  `const-cast-ptr-int.rs` and `static-raw-pointer-interning.rs` leave the
  known failures.
- rustc's suite: `supported-cast.rs`, `coercion-as-explicit-cast.rs`,
  `negative-integer-cast-to-pointer.rs`, `dyn-fn-alignment.rs`,
  `misaligned-reference-drop-field-99838.rs` and
  `deriving-with-repr-packed-3.rs` leave the known failures with stage 1.
  `enum-alignment.rs` asks the address of a number through a `&`, and
  `zst_no_llvm_alloc.rs` that an empty `Vec`'s pointer equal an empty
  slice's and a static's zero-sized value have an address of its own,
  which are unstable guarantees; each stays, saying so. The rest as
  their stage lands.
- The architecture test: `pointers.rs` alone makes a handle, computes an
  address, or reads bytes through a pointer; `refusal.rs` alone words what
  stays refused.
- Mutations, each stage's: an offset dropped, a base shared, an address
  not exposed, a tagged pointer read, `==` by identity alone, a byte
  read big-endian, a width ignored, a prefix cast refused.

## Amendment: what CI found after the five stages (2026-10-21)

rustc's suite, run whole for the first time since stage 1, found two
programs that had been refused and were now wrong, and three that had
passed and now panicked:

- **A value of no size reads and writes through any pointer that isn't
  null**, as Rust's does: a pointer with no place to one gives its JS
  value, `undefined` or `[]`, and a write changes nothing.
  `pointers::at_address` is the one place a pointer at an address is
  given its size and that value; `representation::zero_sized_js` writes
  the value as `const_js` writes a constant of the type, and
  `has_one_value` is the one answer to "has no size", which a second copy
  of it, `is_zero_sized`, had asked too.
- **An empty array is an allocation of no bytes**, dangling as a value of
  no size is: at its item's size, which is a primitive's alignment, and
  an empty slice made of a pointer with no place is at that pointer's
  address. Two pointers into two allocations are equal where their
  addresses are, so `from_raw_parts(NonNull::dangling(), 0).as_ptr()` is
  `NonNull::dangling()`. `$ptrEq` takes the pointee's size, as `$ptrCmp`
  does, and `pointers::pointers_equal` is the one place it's written.
  Of an empty array of a struct whose size isn't its alignment, the
  address is a multiple of the alignment, not the alignment itself: a
  program comparing one with `NonNull::dangling()` of that struct sees
  them differ, which native Rust doesn't promise either (case B). An
  empty array compared with an address before anything asked its own, an
  empty `Vec`'s `as_ptr()` with `NonNull::dangling().as_ptr()`, is given
  that dangling address then, as `$addr` gives it (2026-10-28: it was
  equal to no address, `false` where Rust is `true`; `zero_sized_pointers`).
- **A union's field JS holds as an object is decoded only where it fills
  the union**, its scalars covering every byte (`plain::fills`): the union
  then holds it, so `u.a.0 = x` while it held `b` changes `b`'s bytes, as
  rustc's `union-overwrite.rs` asks. A smaller one, or one with padding,
  stays rust-js's error where it's read, as a write into a copy would be
  lost.
- **A `&'static mut T` result made of a `&'a mut T` argument** by
  `transmute`, as yoke erases a lifetime, is the argument's place:
  `mut_refs::borrow_outlives_call` counts a `'static` reference to the
  borrow's own pointee type, so the argument is given as its live cell,
  not a box copied back before the write.

## Amendment: comparing pointers to a `dyn` (2026-10-22)

Two `*const dyn Trait` made of one `&a` compared `===` as the two pair
objects JS made, `false` where Rust's `==` is `true`: a wrong answer stage
3 brought. `pointers::compared_by_address` is now the one answer to
whether a raw pointer is compared by place or address, a handle on its
place or a `dyn`'s pair, which MIR's `==`, a derived `PartialEq` and its
check of a struct's fields each ask; `pointers_equal` writes two pairs'
`==` as `$dynPtrEq(p, q)`, the same value, and `ptr::addr_eq(p, q)` the
same way. Two pointers `==` can compare are of one trait, so one value's
vtable is one: the vtable isn't compared, which no program can tell.
`<` of two `dyn` pointers is refused. A pair of a value JS holds as
itself, a number's, compares the numbers: two `dyn`s of two equal numbers
are equal, where Rust's addresses differ (case B, as a number reached
through a `&` has no place, above). `dyn_pointers` holds `==` and
`addr_eq`, against native Rust.

## Amendment: `ptr::eq` of references compares places

`ptr::eq` was `===` of its two pointers, which is right of JS objects
(ADR 0285) but not of handles on places: a reference read out of a slot,
`refs[0]` of a `Vec<&Point>`, made a pointer to that slot, `$ptr(refs,
0)`, so `ptr::eq(refs[0], &points[0])` was false (#124). A pointer to
what a reference read from a slot or a field refers to, an object, is
`$ptrTo` of it; and `ptr::eq` of pointers to Rust's values compares as
their `==` does, `pointers_equal`: `$ptrEq`, by place, or of two objects'
`$ptrTo`s, `a === b`, which `==` of them reads as too. Held by
`test/corpus/ptr_eq_through_refs.rs` and the mutations
`ptr-eq-of-handles-identity`, `pointer-of-slot-reference-slot` and
`pointers-of-objects-by-handle`.

## Amendment: a handle's identity isn't read

`p == q` is the same place, by its allocation and where it is in it, and
a `$ptrTo` handle's allocation is the object it's to, so two handles to
one object are equal whether or not they're one: `$ptrTo` keeps one per
object to make none twice, not for `==`. Its mutation,
`ptr-to-uncached`, which nothing caught, is removed (#194).

## Amendment: `Rc::as_ptr`, and two pointers to one field

React's update queue compares two updates by identity,
`a.as_ref().map(Rc::as_ptr) != b.as_ref().map(Rc::as_ptr)`, which was
refused: `Rc::as_ptr` was no pointer operation.

- **`Rc::as_ptr(&rc)` and `Arc::as_ptr` are `&raw const **rc`**,
  `PointerOp::RcAsPtr`: `pointer_to` of what the `Rc` points at, an
  object's `$ptrTo(object)`, a counted `Rc`'s value's place, `$ptr(rc,
  "value", 0)`. So `Rc::as_ptr(a) == Rc::as_ptr(b)` is `a === b`, as
  `Rc::ptr_eq(a, b)` is, and `*Rc::as_ptr(&rc)` reads what it points at.
- **It counts an `Rc` as `ptr_eq` does** (`reads_rc_count`): not one of an
  object, whose identity is the object's (ADR 0582), but one of a number or
  text, whose identity is its box's, `{ value, strong, weak }`.
- **Two pointers to one field are equal where they're of one object**
  (`pointers_equal`): `$ptr(x, "value", 0) == $ptr(y, "value", 0)` is `x
  === y`, as two `$ptrTo`s are, since no two objects share a field. Of
  another field, or another offset, `$ptrEq` compares them. A `static
  mut`'s `ptr::eq(&raw const TOTAL, &raw const OTHER)` is `TOTAL ===
  OTHER` too.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a pointer's identity is
its place's, as Rust's address is.

Held by `test/corpus/rc_as_ptr.rs`, against native Rust, and the mutations
`rc-as-ptr-refused`, `rc-as-ptr-of-counted-box`,
`rc-as-ptr-of-number-uncounted`, `rc-as-ptr-of-object-counted` and
`field-pointers-equal-of-any-field`. An `Option` of two such pointers,
`.map(Rc::as_ptr)`, compares by `$ptrEq` of what each holds: written as
React writes it, `a != b` of two `Option<Rc<Update>>` whose `PartialEq` is
`ptr::eq` is `a != b`, as of any `Option` of what `===` compares (ADR 0864,
amended).

## Amendment: a function's pointer is the function

A pointer to a value of no size was its dangling address, `$ptrFrom(1, 0)`,
whose value is `undefined`, a unit's. A fn item, `noop`, and a closure that
captures nothing are of no size too, but each is a JS function, by its
identity, so its pointer had lost what it points at:
`std::ptr::eq(handler, &noop)`, rustc's `&raw const *&noop` made a `*const
dyn Fn`, was `handler === undefined`, false of `noop` itself, and a `*const
dyn Fn` of `&noop` was `$ptrTo(undefined)`, which panicked where it was made.
A wrong answer, met as React's ReactDOMComponent compares `node.onclick ===
noop` (rd128).

- **A pointer to a function of no size is `$ptrTo` of the function**
  (`pointers::is_function_of_no_size`), as a `dyn Fn`'s is: no address, but
  the function, so `std::ptr::eq(handler, &noop)` is `handler === noop`, and
  `std::ptr::eq(handler, &quiet)` of a closure `handler === quiet`.
- **`has_no_place` is the one answer** of whether a pointer to a type has
  no place, but its address: a value of no size, not a function. A pointer
  made (`dangling_pointer`) and one made a `dyn` (`rvalues`) ask it, so
  neither reads `undefined` for a function.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
the function's identity is kept, as Rust's pointer and its `dyn`'s vtable
keep it.

Held by `test/corpus/fn_compared_by_its_pointer.rs`, against native Rust:
a fn item, two closures that capture nothing and one that does, compared
through a `&dyn Fn` parameter, a thread-local's `Option<&'static dyn Fn>`
and a `*const dyn Fn`; and the mutations `function-pointer-an-address`,
`function-pointer-made-dyn-undefined` and `closure-pointer-an-address`.

## Amendment: `Rc::as_ptr` of an `Rc` an item holds (#1019)

`Rc::as_ptr(&first.0)` of an uncounted `Rc` a tuple struct holds, the
net's `field` rewrite of `rc_as_ptr`, was `$ptr(first, 0)`: `pointer_to`
was given the `Rc`'s read, an array's item, and made a pointer to the
item, the `Rc`'s place, so two clones' pointers differed (native `true`,
JS `false`). What an uncounted `Rc` of an object points at is the object,
wherever the `Rc` is read from: `$ptrTo(first[0])`. A counted one's is
its box's `value`, as it was. Held by the corpus case
`rc_as_ptr_of_a_held_rc` and the mutation
`rc-as-ptr-of-a-held-rc-its-slot`. The net's `field` rewrite of
`pointer_addresses`, the address of a struct a tuple struct's field
holds, `&raw const held.0`, an item's handle where its fields' are the
object's, stays #1019's.

## Amendment: a field of an object that's an array's item is in the array (#1019)

`&raw const held.0` of a struct a tuple struct's field holds is the
array's handle on its item, `$ptr(held, 0)`, whose address is the
array's base; but `&raw const held.0.b` is the object's, `$ptr(held[0],
"b", 4)`, whose address was the object's own base, so their difference
was 4100, where native's is 4. And `transmute(&held.0)`, the object itself
as an address, was its own base too, not `&raw const held.0`'s. An object
an array's item handle was made on is that item (`$items`), so now its
fields are in the array's allocation, at the item's address and the
field's offset, where the array's stride is known; and the object as an
address is the item's. A field's address asked first, before the
array's, `&raw const held.0.c` before `&raw const held.0`, is the
object's own, and the array whose first item it is is put at that base
when its address is asked, where its bytes fit the object's reserved
range. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's are kept. Held by
the corpus case `address_of_a_struct_a_field_holds`, in both builds, and
the mutations `field-of-an-item-its-objects-own-address`,
`field-of-a-later-item-at-the-first`,
`array-of-an-addressed-first-item-apart` and
`reference-to-an-item-its-objects-own-address`. The net's `field`
rewrite of `pointer_addresses` runs as native, taken off.


## Amendment: a tuple's field is at its offset, not its index (#1081)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results changed.
JS holds a tuple and a tuple struct as an array, so a pointer to its
field, `&raw const t.1`, was the array's handle on an item, `$ptr(t, 1)`,
whose address was its index times its size. rustc lays a tuple out as it
likes, reordering its fields: of a `(u8, u32, u16)`, the field addresses
less the tuple's were `0 1 2` where native's are `4 0 6`; of a
`#[repr(C)]` tuple struct a tuple struct's field holds, `&raw const
held.0.1 - &raw const held.0` was 4100 where native's is 4, the field
taken as an item of the inner array, apart from the outer.

**A tuple's field's handle is given its byte offset, as a named field's
is**, `$ptr(t, 1, 0)`, rustc's layout for wasm32 (`field_offset`,
`pointers::Offset`), and the runtime reads a handle with an offset as a
field, never an item: its address is its offset in the allocation that
holds it (`$offsetIn`), and a tuple's field that's an object is at that
field's address in the tuple's allocation, as an array's item is (#1019
above, `$items` keeping the offset), the outer tuple's address given
first. A tuple whose field at offset 0, an object, has an address
already is put at its base, as an array whose first item has one is.
Of a generic tuple, whose layout its caller says, the offset is `null`,
and its address is refused where it's asked, "the address of a field of
a type its caller lays out", as a named field's given none is. Since it's
no item, stepping a tuple's field's pointer, `p.add(1)`, and the
distance between two, `offset_from`, are refused where they run, as of
any pointer not into an array: the layout says which field is next.

Not done then: the address of a field of an object that's an array's
item or a struct's field where no handle on that item or field was made
(#1089, below).

Held by the corpus case `address_of_a_tuple_field`, against native Rust,
`address_of_a_struct_a_field_holds` (whose pair is a tuple now held at
its offsets), `test/runtime-package.test.ts`'s "a tuple's field's pointer
is at its offset, never an item", and mutations
`tuple-field-pointer-no-offset`, `tuple-field-of-a-generic-tuple-an-item`
(`lower/pointers.ts`), `tuple-field-at-index-times-stride`,
`tuple-field-an-item`, `tuple-field-of-a-field-its-own-allocation`,
`tuple-of-an-addressed-field-apart`, `tuple-field-pointer-stepped` and
`tuple-field-pointers-distance` (`runtime/pointers.ts`). The metamorphic
net's tuple-struct rewrite of `address_of_a_struct_a_field_holds` is off
its list.

## Amendment: a nested place's pointer is given its root and its offset there (#1089, the user's decision)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results changed.
A field's pointer was placed in the allocation that holds its object only
where a handle on that object's place was made first (`$items`, #1019,
#1081). Without one, `&raw const items[1].b` beside `&raw const items`
alone, or `&raw const o.inner.b` beside `&raw const o`, the inner object
was an allocation of its own: `4096 8196` where native Rust prints `8 20`.

**A raw pointer to a place reached through more than one field or item
from where its projections start, its local or what its last `Deref`
reads, is given that root and the place's byte offset in it**, rustc's
layout for wasm32, `$ptr(items[1], "b", 0, items, 8)`, `$ptr(o.inner,
"a", 4, o, 4)`; an index that's a variable's scaled by its item's size,
`items, i * 8 + 4` (`place_in_root`). `$addr` of one is the root's
allocation's base and that offset. A place of one step is as it was,
`$ptr(o, "x", 8)` (the user's choice of the options: making the
container's handle first, which leaves an array's stride unknown, or
refusing, were the others). Only what a reference refers to isn't known
where a pointer is made, so a place through one is found as the item or
the field a pointer to it was made of, as before (#1019, #1081).
`address_of_a_struct_a_field_holds` and `address_of_a_tuple_field`, whose
nested places are of this rule, are given their roots.

Held by the corpus cases `address_of_a_nested_field`, against native Rust,
and `address_of_a_field_through_a_reference`, which holds #1019's and
#1081's lookups of a place through a reference;
`test/runtime-package.test.ts`'s "a nested place's pointer is given its
root and its offset there"; mutations `nested-place-unrooted`,
`nested-place-index-unscaled`, `nested-place-field-offset-lost`
(`lower/mir/rvalues.ts`) and `rooted-place-its-objects-own`
(`runtime/pointers.ts`), and #1019's and #1081's retargeted to the
reference case.

## Amendment: a slice of a byte pointer's items is of its type (#1107)

`slice::from_raw_parts(p, n)` of bytes retyped as a wider integer's,
`a.as_ptr() as *const u32`, read a byte for each item, `[1, 0]` where
native Rust's is `[1, 258]`, silently: `$ptrSlice` took the view's bytes
and not its width. Each item is read as the view reads one, `width` bytes
apart, little-endian, signed as its type is, or a `[u8; N]`'s bytes; and
`from_raw_parts_mut` of a byte view is a view of its slots only where
they're its items, unsigned bytes, not an `*mut i8`'s, which is rust-js's
error. Case C. `test/corpus/slice_of_words_through_a_byte_pointer.rs`,
against native Rust; mutations `byte-slice-its-bytes`,
`signed-bytes-mut-unsigned` (found fixing #1099).

## Amendment: a rooted place's root that a temporary holds is made first (#1110)

A nested place whose root is what a `Deref` gives through a call, a
`Ref`'s `(*cell.borrow())[2].a`, a `RefMut`'s, an `Rc`'s, a reference a
function returns, read the temporary holding that reference twice: once
for the place, which took its value where it's used, and once for its
root, `place_in_root`'s, which found it gone and wrote its MIR name,
`$ptr(items[2], "a", 4, _25, 20)`, which no JS declares: a
`ReferenceError` where native Rust prints `20`, since 6226b1d96 (#1089;
before it, the address was the inner object's own, `4100`). Such a
place's temporaries, its root's local or an index's, are made first, a
`const`, as `flush` makes what's pending before what mustn't run first,
so the place and its root each read the one value (`rooted_from`, the one
answer of where a root ends). And an array's item a tuple holds,
`(wrapped.0)[1]`, a handle that says no offset of its own, `$ptr(items[0],
1)`, was never rooted, `4108 4096` where native Rust prints `12 0`: it is
now, its root's offset saying where it is, `$ptr(items[0], 1, undefined,
items, 8)`. Case C. Corpus case
`address_of_a_nested_field_through_a_deref`, a `Ref`, a `RefMut`, an
`Rc`, a `Box` and a returned reference, against native Rust; the
metamorphic net's four `address_of_*` cases rewritten through an
`Rc<RefCell>`; mutations `rooted-place-temporary-unmade` and
`rooted-item-of-no-offset-unrooted`, and `nested-place-unrooted`
retargeted at `rooted_from`.

## Amendment: where an object is stored is checked where it's read (#1113, step 1)

`$items` records where an object is stored by value, its container, key
and offset, which `$addr` follows to the allocation that holds it (#1019).
A record was believed however the object moved since: a `Vec`'s item a
pointer was made to, then shifted by a `remove` before it, was at its old
index, `8` where native Rust's is `0`; one moved out of the `Vec` into a
local of its own was still in the buffer it left. A record is now checked
where it's read: the object is still at its key; else, an array's item, at
the index `indexOf` finds it at now, the record moved there; else it was
moved out, and is its own allocation, as Rust's is where it was moved to.
`indexOf` is linear in the array's length, paid only where an address is
asked of an item that moved since its record was made, once each move,
as the record is moved with it. Case C. Corpus case
`address_of_an_item_moved_in_its_vec`, against native Rust; mutations
`stored-object-record-unchecked` and `stored-item-index-unfound`. The rest
of #1113, recording where an object is stored as it's stored, follows.

## Amendment: an object says where it's stored, recorded where it's borrowed (#1113, the user's decision)

A field's pointer through a reference to an object stored in another,
`let r = &(other.0).0; &raw const r.c`, was placed in that object's own
allocation, while `&raw const (other.0).0`, rooted statically (#1089), was
in `other`'s: `4294963208` where native Rust prints `8`. Which allocation
an object's address was in depended on which pointer asked first, and
whether its whole path from a local was in one statement.

The user chose (2026-11-09) to record where an object is stored, for the
types whose addresses are asked alone. The record is `$items`', the
container, key and offset that `$addr` follows to the outermost
allocation, checked where it's read (step 1, above). It's written where a
reference or a raw pointer to a place is made, `$stored($ptr(other, 0,
0))`, of each field or item the place passes through from its last
`Deref`, and `$storedItems(v)` of a `Vec`'s or an array's borrowed whole.

**Why where the borrow is made covers every address.** Every address is a
pointer's; every pointer is made of a place, by `&raw`, or of a reference,
by a cast; and every reference is made of a place too, by `&`, whose path
MIR sees there, from a local, an allocation of its own, or from a `Deref`
of another reference, whose referent was recorded where that reference was
made. So each object a pointer is in was recorded on the way to it. The
ways a reference reaches a program without a borrow of its whole path in
the crate, and why each is still recorded:

- std's references into a `Vec` or an array, `iter`, `iter_mut`, `get`,
  `get_mut`, `first`, `last`, `split_at`, `chunks`, `as_ptr`: std is given
  a borrow of the `Vec`, `&v` or `&mut v`, which records its items,
  `$storedItems(v)`, so each item it lends is one recorded. A shared
  subslice is a copy of its items (ADR 0063), and an item of one isn't
  recorded as stored there: its `Vec`'s record stands. Corpus case
  `address_of_an_item_std_lends`.
- A `HashMap`'s or a `BTreeMap`'s `get`: its values are stored in its own
  allocation natively, in an order its hasher or its tree decides, which
  no program reads an address of; a JS `Map`'s value is an allocation of
  its own. No container is recorded, and none is asked of.
- `Box::into_raw`, `Box::leak`, `Rc::as_ptr`: a box's or an `Rc`'s value
  is an allocation of its own natively, as its object is in JS; an object
  stored in it, as anywhere, is recorded where it's borrowed.
- A closure's capture: by reference, a borrow made where the closure is,
  whose place is recorded there; by value, the object moved into the
  closure, which is where it's stored, as Rust's is.

**Which types.** A crate that asks no address, `p as usize`, `p.addr()`,
`offset_from`, `p < q`, `{:p}` or `{:?}` of a pointer, records nothing:
`ptr::eq` and reads through a pointer ask none (`asks_addresses`). One that
does records the types of each object a raw pointer's place passes through
or points at, and each struct, enum or union that holds one of those by
value, again till none is added; a type parameter's value, where any is,
as it may be one; and every tuple and array, objects of no definition a
record could name (`StoredPlaces`, `analysis/stored_places.rs`, the one
owner). An array's item, a tuple's part, are held by value; a `Box`'s, an
`Rc`'s and a `Vec`'s items aren't, each an allocation of its own.

**Moves.** An object moved since a record was made is where it's stored
now: a record is checked where it's read, and one whose object isn't at
its key any more is dropped, the object its own allocation till a borrow
of where it is now records it again. A borrow of a `Vec` records its
items again, so step 1's `indexOf` of an item a `remove` shifted is
gone: a borrow records it where it is. A rooted handle (#1089) stands only
where its object's records lead to its root still, else where they
lead: an object swapped into another container, `mem::swap(&mut a.inner,
&mut b.inner)`, is in `b`'s. With records, the rooting says what they
do; it stays, as the form of a place whose root is in the statement, its
mutations now of that form, `snapshots`.

**Strides.** An array's item is recorded with its size, where it's known,
so its address is the array's and its index times that, the array's
own address found as any object's is: an array a tuple holds, its items
reached through references.

**A `Vec` is no container.** Its items are an allocation of their own
natively, wherever the `Vec` is: a `Vec` a struct holds isn't recorded
as stored there, though its items are recorded as its.

**Not covered.** A variant's field, an enum's payload, whose offset its
enum's layout doesn't say, is its object's own allocation. A reference a
library makes into its own type's object isn't recorded unless the
library asks addresses itself: whether a consumer's address of such a
field is refused, or libraries record their public types' stores, is the
user's to decide (#1113's step 4).

**Cost.** Only in a crate that asks an address: a `$stored(..)` beside a
borrow of a recorded type's place, and an O(n) `$storedItems` beside a
borrow of a whole `Vec` of them, once a borrow, not once an item; and an
`indexOf` where an item moved since its record (step 1). Case C. Corpus
cases `address_of_a_field_where_its_object_is_stored` (the #1113 repro
with its address asked before its container's, through a call, returned,
a `Vec`'s items through its iterator, a move between containers, two deep,
and an object's own address asked first) and
`address_of_an_item_std_lends`, against native Rust; the metamorphic net's
`address_of_*` cases through every rewrite, #1113 off its known list;
mutations `stored-at-reference-unrecorded`, `stored-items-unrecorded`,
`stored-slice-item-recorded`,
`stored-places-unasked`, `stored-places-cast-unseen`,
`stored-places-unclosed`, `stored-root-kept-after-move`,
`stored-item-stride-unread` and `stored-record-unwritten`;
`rooted-place-temporary-unmade` retargeted at the one helper that makes
a place's temporaries first, `read_more_than_once`, which corpus case
`pointer_into_a_borrow_read` holds in a crate that asks no address.
With records, the runtime's rooted branch in `$addr` gives what the
records give wherever its root's chain holds: `rooted-place-its-objects-own`
no longer changes any output and is removed, the branch kept as the
shortcut it is. So do the heuristics for an object no record names, a
first item's or field's address asked before its container's, a later
item's at its index times its array's stride, `$ptrTo` of an item at its
place (#1019, #1081): every tuple, array and recorded type is recorded
where it's borrowed, so none changes an output the corpus, the iterator
tables or the metamorphic net has; their mutations,
`tuple-of-an-addressed-field-apart`, `field-of-a-later-item-at-the-first`,
`array-of-an-addressed-first-item-apart` and `item-pointer-not-placed`, are
removed. The code stays for what no record names yet, an enum's payload,
whose records #1114 makes, and goes with them. The lowering's code of where a place's objects are
stored and rooted is one module, `mir/stored.rs`, beside `rvalues.rs`.

## Amendment: what a container holds inline is inside it (#1114, the user's decision)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results changed;
D, refused, where one JS can't say.

std lends references into its receiver's own storage: a `RefCell`'s value
past its borrow flag, a `Mutex`'s or an `RwLock`'s past its state, a
`Cell`'s, an `Option`'s payload past its tag, and a pattern lends a
variant's field. JS keeps each apart, in a `{ value }` cell's `value`, or
as the object itself, which was an allocation of its own: of a
`RefCell<u32>`, `&raw const *n.borrow()` less `&raw const n` was `0`, of a
`Mutex` `4096`, of a variant's field `4100`, where wasm32 gives `4`, `4` and
`8`. And JS holds an `Option` of an object, and a `RefCell` of one (ADR
0863), as the object, so a pointer to the container made of a reference to
it and one to what it holds were one handle, `0` where wasm32 gives `4`.

**What's inside is at its offset there, rustc's layout of the instance,
never a number written here.** One owner, `lent.rs`, says where: of each
`{ value }` container std lends from, the path of its fields by std's
names to what it holds (`HELD_INLINE`: `RefCell` and `Cell` `value.value`,
`Mutex` and `RwLock` `data.value`; an atomic's one field), and of each
value, where in it the object its JS is sits (`object_within`): an
`Option`'s payload's offset, of one that's its payload; a `RefCell`'s
value's, of one that's the object it holds; a type's that's the value it
holds, its value's; an untagged enum's variant's, where each is at one;
nowhere, of a box's, an `Rc`'s, a reference's or a collection's value,
which is an allocation of its own.

- **Records say `within`.** A record of where an object is stored (#1113)
  says where in its slot it is, `$stored($ptr(o, "value", 0), undefined,
  4)`, and `$addr` adds it: an `Option`'s payload in a field, a tuple, a
  local held in a cell of its own, matched or lent by `as_ref`, `as_mut`
  and `iter`; `get_or_insert` and `insert` record the payload they write
  before they lend it. A variant's field is recorded at its variant's
  offset (`for_variant`), as a struct's is at its own.
- **A `{ value }` container records where it keeps what it lends where
  std's call is given it**, `$borrow($storedValue(cell, 4))`, the cell
  itself, never throwing, so the record stands where the call is, its
  object and a handle on its `value` at that offset (the form the
  coordinator chose, for the user, over a statement before the call,
  which was emitted twice and broke tidy's reads around it). A struct's
  `RefCell` field of a number, which holds it (ADR 0863, amended), records
  it where it's borrowed for such a call, `$storedIn(counter, "count", 0,
  4)`, the field's offset and the value's in it, and a pointer to what it
  lends is a handle of its own on the field, `$ptrIn`; a pointer to such a
  field is the field's place, not its cell's.
- **A pointer to a container JS holds as the object it holds, made of a
  reference to it, is a pointer to what holds the object**,
  `$ptrOuter(object, 4)`: its address is the object's less the offset; it
  reads and writes the object, as `$ptrTo`'s does; where the object's
  record says it's that far into its slot, it's the slot's place. And a
  pointer to what `get_or_insert` or `insert` lends, whose JS reads the
  `Option`'s place, is the object's, `$ptrTo`, not a handle on that place
  (`lent_from_a_holder`, the MIR's call that made the reference); so is
  one to what an `Rc`'s or an `Arc`'s `deref` lends, which JS reads as the
  field holding it, `s[2]` of a tuple struct, whose handle was inside the
  struct, `false` where native Rust's is outside it. Each
  allocation keeps 64 bytes past its own, so what holds an object no slot
  is recorded as holding is in no other allocation. Of an `Option` holding
  `None`, which JS holds as `undefined`, no place, it's refused where it
  runs, saying to point at it where it's stored (#1132, open for the user:
  the alternative is refusing such a pointer as it's made, of a `Some` too).
- **What's an allocation of its own isn't recorded where it's held.** A
  `Box`'s, an `Rc`'s or a `Vec`'s value was recorded as stored in the
  field holding it, since #1113, so `&raw const s.b` and `&raw const *s.b`
  were one address, `true` where native Rust gives `false`: a wrong answer
  of its own, found here, now none (`address_of_what_a_box_holds`).

**Generic code** has one JS for every instance of its type parameter. A
pointer made of a reference to a `T`, and a record of a `T` stored in a
slot, take the offset every instance the crate gives it agrees on
(`StoredPlaces::instances`, #1009's `instances_of`, now `instances.rs`'s);
where they differ, an `Option` of an object and an object, it's refused,
naming #1114, in a crate that asks an address: "make it where the type is
known". Where the crate can't see every instance, a `pub` function's, it's
0, the offset of every type but those.

**Every other API that lends from inside its receiver**, a std function
whose first argument is a reference to a std type and whose output holds
that reference's lifetime, `OnceCell::get`, `Result::as_ref`,
`Peekable::peek`, where rust-js doesn't know where its receiver keeps it,
is refused in a crate that asks an address of what it lends (a recorded
type, or what one of its raw pointers points at), naming #1114
(`check_lent`). A pattern's place, `if let Ok(x) = &r`, is a variant's
field, recorded as above.

**The record-less heuristics go.** With every container's records, the
runtime's guesses for an object no record names, a first item's or
field's address asked before its container's, a later item's at its index
times its array's stride where its record had none, change no output
the corpus, the iterator tables or the metamorphic net has, and are
removed; an item of an array that's itself recorded, a cell's `value`
say, is at the array's address and its index times its size. A handle
made on an array's item, a tuple's field, recorded the object there too,
which made a tuple struct's `Rc` field's value in the struct's
allocation (the net's `tuple-struct` rewrite of
`address_of_what_a_box_holds`): it records nothing now, and `==` of an
object's pointer and a slot's holding it, where nothing recorded it, is
`true`, which `thin == &raw const held.0` of a `dyn`'s pointer made thin
needs in a crate that asks no address (the net's `field` rewrite of
`dyn_pointers`). Mutations whose code moved are
retargeted: `stored-record-unwritten`, `tuple-field-at-index-times-stride`,
`field-of-an-item-its-objects-own-address`,
`reference-to-an-item-its-objects-own-address`, `ptr-to-written-whole`,
`address-items-not-strided`, `first-item-offset-by-stride`,
`stored-slice-item-recorded`, `pointer-pointee-object-by-place`,
`write-result-field-of-err` (now of `field_is_its_base`, the one answer of
which field is its place's own JS value), and `rc-param-impl-method-resolved`
and `rc-public-callers-seen`, moved with their code to `instances.rs`.

**Cost.** Only in a crate that asks an address: a `$storedValue(..)`
around each receiver of a call that lends from a `{ value }` container, a
`$storedIn(..)` before a borrow of a `RefCell` field for one, and a
`within` argument on records of slots of `Option`s of objects. A crate
that asks none emits none of them (`pointers_into_a_cell_compared`).

Held by corpus cases, run natively on wasm32, whose `usize` is
rust-js's: `address_of_what_a_cell_lends`, `address_of_an_option_payload`,
`address_of_a_variant_field`, `address_of_what_a_box_holds`, the refusals
`address_of_a_generic_option_refused` and
`address_lent_by_a_once_cell_refused`, and `pointers_into_a_cell_compared`
beside them; the metamorphic net's pointer cases through every rewrite;
mutations `lent-cell-unrecorded`, `lent-at-the-cells-start`,
`outer-pointer-the-objects`, `option-payload-at-its-tag`,
`value-cell-at-its-flag`, `box-held-in-its-field`,
`generic-within-unrefused`, `uncatalogued-lender-allowed`,
`crate-lender-refused`, `variant-field-unlaid`,
`holder-lent-pointer-the-slots`, `rc-lent-pointer-the-slots`,
`option-inserted-item-strideless` (`lent.rs`);
`stored-within-dropped`, `stored-held-local-unrecorded`,
`stored-property-cell-unrecorded`, `stored-variant-field-unrecorded`
(`mir/stored.rs`); `option-inserted-unrecorded` (`std_types/option.rs`);
`stored-value-offset-unread`, `stored-value-object-unrecorded`,
`outer-at-its-object`, `outer-placed-at-any-depth`, `within-unread`,
`field-content-handle-the-slots`, `item-of-a-recorded-array-its-own`,
`container-exposed-on-the-way`, `slot-of-its-object-apart`,
`ptr-at-of-a-recorded-array-placeless` and
`ptr-eq-of-a-recorded-handle-unplaced` (`runtime/pointers.js`). The net
found the last four: an address found again, `n as *const T`, was the
container's slot, which `$addr` exposed on its way to the item's at the
same address (an ask on the way to another's, `inner`, exposes nothing
now); and `with_addr`, `wrapping_byte_add` and `==` of an item's pointer
into an array a cell holds, which has no addresses of its own, found no
place where its record puts it. The net runs a case run natively on
wasm32, `//@ native: wasm32`, as the corpus does, its rewrites' native
side on wasm32 too, its library rewrite's crate built for it: the cases
here, whose offsets are a `usize`'s, are in it. The guide's limitations page says the two
refusals.

## Amendment: `write_bytes`, and an array's bytes through its items' codec (2026-11-10, #1133, #1137)

`ptr::write_bytes(dst, byte, n)` and `dst.write_bytes(byte, n)` were
refused as addressable memory. getrandom 0.3's `uninit_slice_fill_zero`
zeroes a `MaybeUninit<u8>` buffer so, and its `fill` was then types alone.
**They're `$ptrFill(bytes, byte, n * size)`, of `dst` seen as bytes**
(`recast` to `u8`): a byte's or a `MaybeUninit` byte's place as it is,
and plain data's, integers, floats and `#[repr(C)]` structs, through
their codec, as wasm32 lays them out. A type with no codec, a reference
or a `String` say, is refused, "writing bytes into a `T`". getrandom's
form prints `$ptrFill($ptr(slice, 0), 0, slice.length)`. Case N of
ADR 0262: the items hold what native Rust's do.

**A wrong answer found with it (#1137, P0).** A byte written through a
pointer into an array of signed integers read back unsigned. `[0i16; 2]`,
its first two bytes `0xFF`, was `[65535, 0]` where native Rust's is
`[-1, 0]`. `$ptrBytes` kept a plain type's codec for one value's bytes
but dropped it for an array's, and `$byteSet` decoded the item it changed
with `$fromBytes(.., signed: false)`, whatever its type. An array of
`i64`s with a negative item, or of floats, read wrong too. **An array's
byte view keeps its items' codec**, and `$byteAt` and `$byteSet` read and
write an item through it. One with no codec, of an allocator's slots, is
read as before.

Held by the corpus case `ptr_write_bytes.rs`, against native Rust: each
form, `u8`, `MaybeUninit<u8>`, `u32`, `i16`, `u64`, `i64`, `f32`, a
struct and an array of them, a byte view over wider items, a count of
zero; and the signed array written a byte at a time. Also by the
mutations `write-bytes-of-items-not-bytes`, `ptr-fill-of-none` and
`array-byte-set-unsigned`, each caught.

## Amendment: what a call lends is read of its declared signature (2026-11-10, #1199)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, a refusal lifted;
what's lent and what's refused is otherwise as #1114 said. Whether a std
call lends from inside its receiver, and what (`lent_from_receiver`), was
read of its signature as the caller's types make it, every lifetime
erased alike, so a lifetime the caller's types bring looked like the
receiver's own: `it.by_ref().skip_while(|&x| { calls += 1; x < 2 })`,
whose `Self` is a `&mut IntoIter<u32>` and whose closure captures a `&mut
u32`, looked as if it lent a `u32` from inside the iterator, and was
refused in any crate that asks an address anywhere, as the neighbour
shuffle found beside `address_of_a_nested_field.rs`. It's read now of the
signature std declares, where `skip_while` takes `self` and lends
nothing, `OnceCell::get(&self) -> Option<&T>` lends its `T`; then the
caller's types are put in, their lifetimes erased. So are the raw
pointers it lends, the declared ones, `Cell::as_ptr`'s: a raw pointer
the caller's type puts in what's returned, a `Cell<*const P>`'s `get()`,
is a copy out, lending nothing of the cell's.

- **Tested** by the corpus case
  `adapter_of_a_borrowed_iterator_where_addresses_are_asked.rs`, a
  `skip_while` and a `take_while` of `by_ref()` with closures that borrow,
  collected, folded and unwinding, beside addresses asked, in both builds
  against native Rust; `address_lent_by_a_once_cell_refused.rs` is still
  refused; mutation `lent-lifetime-of-the-callers-types`.

## Amendment: an `i8`'s bytes, `offset_from` across a byte view, and a `MaybeUninit` array's items (2026-11-10, #1253, #1254, #1255)

Three wrong answers the memory net (#1172) found on its first run, each a
P0, ADR 0262's "results change" case, Rust's behaviour kept:

- **An `i8` array's items aren't its bytes (#1253).** A pointer to `i8`s
  was taken for one to bytes, as a `u8`'s is, so `slice::from_raw_parts(
  a.as_ptr().cast::<u8>(), n)` of `[-21i8, ..]` was `[-21, ..]`, native
  Rust's `[235, ..]`, and a `u8` written through it kept its number,
  `235` in an `i8` array. Only a `u8` is a byte: an `i8`'s pointer goes
  through its codec, `$ptrBytes`, as any other integer's (#1137), and to
  an `i8` from bytes is `$ptrRetyped(view, 1, true)`. Whether a view's
  items are its bytes is the runtime's one question, `$itemsAreBytes`,
  of a `u8` array's view, no codec, which each byte read, write and the
  mutable slice of bytes ask; an `i8` written through a byte pointer is
  its byte, `value & 0xff`. A pointer to `[i8; N]` from bytes is refused,
  as a pointer read as another type's, where it read the bytes unsigned.
- **`offset_from` of a pointer stepped in bytes and cast back (#1254).**
  `p.cast::<u8>().add(8).cast::<u64>().offset_from(p)` is a view of the
  array's bytes and a handle on its item: the distance was the view's
  byte offset less the item's index, `8`, native Rust's `1`. Each is
  placed in the array's bytes, the view by its own offset and the item
  by its index times the stride the view read the array by
  (`$byteOffset`), and their distance is in the view's width.
- **A `MaybeUninit<[T; N]>` written through its item pointer (#1255).**
  `m.as_mut_ptr() as *mut u32` of a `MaybeUninit<[u32; 2]>` was the
  first item of `undefined`, and the write threw a `TypeError`. A
  `MaybeUninit` of an array of any items is its slots, none written yet,
  `Array.from({ length: n })`, as one of `MaybeUninit`s was (ADR 0417),
  so each item is written where it is; one written whole replaces them.
  A `Box` made before its value, `vec![..]`'s `Box::new_uninit`, is
  written whole, and stays `undefined` until it is.

Corpus cases `pointer_cast_to_the_other_sign.rs`,
`offset_from_of_a_pointer_stepped_in_bytes.rs` and
`maybe_uninit_array_written_through_its_pointer.rs`, `//@ native:
wasm32`, both builds, each Miri-clean; the memory net's 400 seeds of its
first run run as Rust does but those its refusals stop. Mutations
`signed-bytes-are-bytes`, `signed-items-are-bytes`,
`signed-byte-written-unmasked`, `offset-from-of-a-view-and-an-item`,
`byte-offset-of-an-item-unstrided` and
`uninit-array-of-plain-items-undefined`.

## Amendment: a byte view of a stored array is where its items are (#1262)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's are kept.

An array stored by value in another object, a tuple struct's field or an
array's item, is placed in its container's allocation once a pointer into
the container is taken (#1113): its items' handles ask `$itemOf` of the
array. A view of its bytes, `p.cast::<u8>().add(3)`, didn't. `$addr` gave
the view an allocation of the array's own, so `q as usize - p as usize`
was `4294963203` where native Rust's is `3`, and `byte_offset_from` across
the two forms was as wrong. **A view of a stored array's bytes is at its
byte offset past the array's first item**, which the record places, so
both forms are in one allocation whichever is asked first. The
metamorphic net's `field`, `array`, `tuple` and `rc-refcell` rewrites of
#1256's `byte_offset_from` case found it. Corpus case
`address_of_a_byte_view_of_a_stored_array.rs`, `//@ native: wasm32`, both
builds; mutation `byte-view-of-a-stored-array-unplaced`.

## Amendment: a slice's pointer range, `swap_nonoverlapping` and `byte_offset_from` (#1256)

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. Each
does what Rust's does over the model above, and native wasm32 agrees in
both builds. They were refused, and they stopped about half the memory
net's programs.

- **`as_ptr_range()`, `as_mut_ptr_range()`** of a slice is the range of
  its first item's pointer to one past its last, made as `start..end` is,
  `{ start: $ptr(items, 0), end: $ptr(items, items.length) }`. The slice
  is spilled where it isn't a plain read, so it's evaluated once. The end
  is the handle one past the last item, which `add` reaches and `==`
  meets, so hashbrown's and smallvec's cursor over the two ends walks as
  Rust's does. One of text's bytes, a copy, is refused, as its pointers
  are read from the range and can't be followed (ADR 0126).
- **`ptr::swap_nonoverlapping(a, b, n)`** is `$ptrSwap(a, b, n)`.
  `ptr::swap`'s helper takes a count, as `$ptrCopy` does, and swaps each
  of the `n` items along its array or its bytes' view.
- **`p.byte_offset_from(q)`** is the difference of their addresses,
  `$addr(p, size) - $addr(q, size)`, each asked by its own pointee's size.
  A byte offset is that difference in one allocation, of any two types, at
  any byte, and of two fields of one struct, and `asks_addresses` already
  makes a crate that calls it record where its places are. The
  alternatives were `$ptrOffsetFrom(p, q) * size`, right only for one
  type, and std's body, both cast to bytes. Neither placed two fields of
  a struct, which stopped where they ran (chosen 2026-11-10).
  Its sign is kept, and like `p as usize` it exposes both addresses.

Corpus cases `slice_pointer_range_walked_by_a_cursor.rs`,
`items_swapped_nonoverlapping.rs` and
`byte_offset_from_is_an_address_difference.rs`, `//@ native: wasm32`, both
builds, through `METAMORPHIC=all`, whose rewrites found #1262. The memory
net's seeds 1 to 400 then ran as Rust does, but for 50 programs in each
build that #1257 stops where they run; none were wrong.

## Amendment: a `MaybeUninit<[T; N]>` written whole is its value (#1258)

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. This
changes only how the JS reads.

#1255 made a `MaybeUninit` of an array of any items its slots,
`Array.from({ length: n })`, so its pointer may write it an item at a
time. One only written whole then read `let m = Array.from({ length: 2
}); m = [7, 8];` where it had read `const m = [7, 8]`. **A
`MaybeUninit<[T; N]>` whose every use is a whole write, `m.write([..])`
through a reference, or a whole read, `assume_init()`,
`assume_init_ref()` and `assume_init_read()`, of its local or a copy of
it, is `undefined` till it's written**, as before #1255. Anything else,
a pointer to it or a reference given to another function, which may
write it through `as_mut_ptr()`, keeps the slots. The fact is the call's,
`Call::written_whole`, found from the body's MIR where the call is
lowered (`uninit_written_whole`). An array of `MaybeUninit`s keeps its
slots whatever its uses, as arrayvec's `uninit().assume_init()` reads it
before any item is written. Corpus case
`maybe_uninit_array_written_through_its_pointer.rs`, given a function
that writes it through its pointer and one written whole and read by
reference. Mutations `uninit-written-whole-unasked`,
`uninit-written-whole-given-anywhere`, `uninit-written-whole-of-no-write`
and `uninit-written-whole-named-is-used`.

## Amendment: a mutable slice of another type's bytes writes where they are (#1257, #1266)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved. For #1257 the answer was a refusal; for
#1266 it was wrong.

`slice::from_raw_parts_mut` of a byte view of another type,
`a.as_mut_ptr().cast::<u8>()` of a `[u32; 2]`, was refused where it ran.
The memory net met it in about one program in eight. **It is a view of
those bytes `width` at a time**, `$proxy(n, ..)`, as a `&mut v[a..b]` is
a view of its items (ADR 0335). Each item is read and written through the
byte view stepped to it, `$ptrAdd(view, i).value`, so through the words'
codec, little-endian as wasm32 holds them. JS's own `reverse`, `fill`,
`sort` and `copy_from_slice` write the words. Where it is goes under
#1185's record of a part's place, as `{ bytes: view }`. A pointer to one
of its items, `$ptr(slice, i)`, is the view stepped to that item, and a
part of it, `&mut v[2..]`, is recorded at the view stepped to its start.
So `bytes[2..].as_mut_ptr()` writes the words and is at their address.

A `[u8; N]` read through such a view, `p.cast::<[u8; 2]>()`, was a fresh
copy of its bytes, so a write into it, `(*p)[1] = 9` or through `&mut *p`,
was lost (#1266, found checking #1257's siblings). **It is a copy of those
bytes whose every write is also a write of the byte it was read from**,
`$bytesRead(view)`. Reads are of the copy, so a pair read and kept, or the
one `swap` holds while it writes its place, is what it was, as Rust's copy
of a `[u8; N]` is. Only a write through the place it was read from reaches
it: what's read and kept, then changed in place, is copied as any read of
such a type is (`copies.rs`). A pointer into it is at the bytes it was
read from. A mutable slice of `[u8; N]`s over another type's bytes is
made of these.

Corpus cases `mutable_slice_of_another_types_bytes.rs` and
`byte_array_written_through_another_types_bytes.rs`, `//@ native:
wasm32`, both builds, through `METAMORPHIC=all`. The memory net's known
list loses #1257's line. Mutations
`mutable-slice-of-another-types-bytes-refused`,
`item-of-a-byte-slice-unplaced`, `part-of-a-byte-slice-unplaced`,
`byte-array-read-written-to-its-copy`, `byte-array-read-live` and
`byte-array-read-unplaced`.

## Amendment: an unwritten place's bytes, and a slice of one place (#1279, #1280)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved.

#1257 let a mutable slice of a `MaybeUninit<u64>`'s bytes write them, as
getrandom's `inner_u64` fills one. Its first write encoded the place's
`undefined` and threw "Cannot convert undefined to a BigInt" (#1279, a P1
found by the Crater). Before #1257 it had been refused where it ran.
**A byte view of a place not yet written starts from zeroed bytes**
(`$encoded`). Nothing can read those zeros: reading uninitialized bytes is
undefined behaviour. This covers every codec width, signed and unsigned,
`u8` to `u128`, and the floats.

Its `MaybeUninit<u8>` case also showed that a mutable slice of one place
that isn't an array's item, `slice::from_raw_parts_mut(&raw mut x, 1)`,
was `$ptrItems`' copy, so a write through it was lost (#1280, a P0 found
by audit). **It is a view of that place, written through its handle.**
Where it is goes under #1185's record as `{ first: pointer }`, the record
#1257's byte slices now share, renamed from `{ bytes }`. A pointer into it
is that pointer: `p.add(0)` of any pointer is the pointer itself.

Corpus case `uninit_bytes_written_one_by_one.rs`, `//@ native: wasm32`,
both builds, through `METAMORPHIC=all`. The memory net's seeds 1 to 400
and 5000 to 5399 still run as Rust does. Mutations
`uninit-place-bytes-encoded`, `slice-of-one-place-a-copy` and
`slice-of-one-place-unplaced`.
