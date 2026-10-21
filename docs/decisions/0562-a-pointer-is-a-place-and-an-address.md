# 0562. A pointer is a place and an address; bytes are plain data's

Status: Accepted, delivered in stages: each stage below names the tests
that hold it, and a stage not yet landed is listed under Consequences as
still refused. Amends [0390](0390-addressable-memory-is-refused-as-such.md),
[0407](0407-a-raw-pointer-is-a-handle-on-its-place.md),
[0408](0408-plain-data-is-its-bytes.md) and
[0406](0406-a-union-is-the-field-it-holds.md); extends
[0417](0417-an-allocation-is-an-array-of-slots.md) and
[0285](0285-ptr-eq.md). Replaces the draft `bytes-through-a-pointer`.

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
  undefined behaviour in the Rust reaches (case A). The five helpers
  that answered this by kind, `$ptrFirst`, `$ptrWhole`, `$ptrBytes`,
  `$ptrTyped` and `$ptrRecast`, become this one rule, asked of rustc's
  layout: `pointers::relayout` is its one owner.

### Fat pointers

- **A `*const dyn Trait` is the `dyn`'s pair**, `{ value, impl }`, as a
  `Box<dyn Trait>` and an `Rc<dyn Trait>` are (ADRs 0331, 0520), and a
  `*const [T]` is the slice (ADR 0407). A thin pointer unsized to one,
  `p as *const dyn Trait`, is the pair of its place's value and the
  dictionary the cast names; a fat one made thin, `p as *const U`, is
  `$ptrTo(pair.value)` of an object. `*p` is the pair, called through its
  dictionary. anyhow's `Box<dyn Error>` taken apart and put back is this.

### Constants and statics

- **A pointer rustc's const-eval made** is lowered from the allocation it
  points into: a static's, `$ptrTo` or `$ptr` of the static by the
  field rustc's layout puts at its offset; an interned value's, `&3`'s,
  a cell of the module's, made once, as a constant's place is (ADR 0407);
  a dangling one, an address with no place.

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
3. **Fat pointers and relayout**: `*const dyn Trait` as the pair, the
   first-field and prefix rules, the five helpers folded into
   `relayout`. `dyn_pointers` and `pointer_relayout`; anyhow and semver
   measured.
4. **Constants and statics** holding pointers. `pointer_constants`.

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
