# 0417. An allocation is an array of slots

Status: Accepted. Extends [0407](0407-a-raw-pointer-is-a-handle-on-its-place.md),
whose handles point into one, and [0332](0332-maybe-uninit.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what's allocated,
written, read, grown and given back as a `Vec`; D for bytes read or written
through a pointer to another type, rust-js's error where it's done; A for
what JS's collector frees, which no program can see.

## Context

A collection that keeps its items on the heap itself, smallvec's spilled
`SmallVec`, hashbrown's table, asks the allocator for memory:
`alloc(layout)` gives a `*mut u8`, which it casts to its items', writes
through, grows with `realloc`, gives back with `dealloc`, and may hand to a
`Vec` with `Vec::from_raw_parts`. Each was refused, and so was a pointer
cast to `*mut u8` or from one: JS has no bytes to point at.

## Decision

**An allocation is a JS array of slots. `alloc(layout)` gives a pointer to
its first slot as bytes, a view, which a cast to the items' type gives back
as the slot's handle (ADR 0407).**

```rust
let ptr = NonNull::new(alloc::alloc(layout)).unwrap().cast::<T>();
ptr::write(ptr.as_ptr().add(len), value);
let grown = alloc::realloc(ptr.as_ptr() as *mut u8, old, new.size());
```

```js
const ptr = $ptrTyped($alloc(layout));
$ptrAdd(ptr, len).value = value;
const grown = $realloc($ptrBytes(ptr), old, new$.size);
```

| Rust | JS |
|---|---|
| `alloc(layout)` | `$alloc(layout)`: a new array's first slot, as bytes |
| `realloc(p, layout, size)` | `p`: the same array, which grows as it's written |
| `dealloc(p, layout)` | nothing: JS's collector frees it when nothing holds it |
| `handle_alloc_error(layout)` | `memory allocation of N bytes failed`, the end, as Rust's |
| `p as *mut u8`, `p.cast::<u8>()` | `$ptrBytes(p)`: a view of its bytes, one for each |
| a view as `*mut T` | `$ptrTyped(view)`: the pointer it was a view of |
| `Vec::from_raw_parts(p, len, cap)` | `$vecFromRaw(p, len)`: the array `p` is the first slot of, `len` long, what's past it left there as `set_len` leaves it ([0420](0420-what-set-len-leaves-stays.md)) |
| `Layout::from_size_align(s, a)` | `$layout(s, a)`: `Ok` of `{ size, align }`, or `Err` as Rust's |
| `layout.size()`, `layout.align()` | `layout.size`, `layout.align` |
| `{:?}` of a `Layout`, a `LayoutError` | Rust's: `Layout { size: 24, align: 8 (1 << 3) }`, `LayoutError` |

- **A view of bytes isn't read, written or stepped through**: JS has no
  bytes, and a slot's value isn't them. Doing so is rust-js's error where
  it's done, as reading a union's field it doesn't hold is (ADR 0406). A
  pointer to bytes that are bytes, a `Vec<u8>`'s, cast to another type's,
  is too.
- **A cast to bytes or from them is always the view's**, in generic code or
  not: a pointer an allocation gives, `*mut u8`, is the program's to cast.
  Another cast between two types' pointers is still a compile error, or,
  in generic code, `$ptrRecast`, rust-js's error where it's made. But a
  whole array's pointer as its first item's, of a type each of whose impls
  of the item's trait is an array of it, smallvec's `*const A` as
  `*const A::Item`, is `$ptrFirst`: decided where it's compiled, as a tuple
  struct is a JS array too, which the runtime can't tell apart.
- **`MaybeUninit::<A>::uninit()` of a type parameter that's an array**,
  each of whose trait's impls is one, smallvec's `A: Array`, is an array
  of slots, none written yet, `[]`, as an allocation is; of a
  `MaybeUninit<A>` too, which is `A`'s value (ADR 0332). Decided where it's
  compiled, as `$ptrFirst` is (`pointers::is_array_param`).
- **What std's functions are is std's own path**, `recognition::std_path`,
  of a crate that names `alloc` its own way too, hashbrown's `extern crate
  alloc as stdalloc`, through which rustc names them, `stdalloc::alloc::
  alloc`, refused before (`test/lowering.test.ts`).
- **`alloc_zeroed`** is refused still: what a zero is depends on the items'
  type, which the allocation doesn't know.

## Why

- **It's the program Rust runs**: `test/corpus/allocator.rs`, a buffer of
  `String`s grown by `realloc`, popped, given to a `Vec` and dropped, and
  one of numbers given back by `dealloc`, prints what native Rust does.
- **It's what a JS developer reads**: an array, grown by writing to it, as
  a JS developer's own buffer is.

## Consequences

- A pointer cast to bytes and stepped, `p.cast::<u8>().add(n)`, as
  arrayvec does for a type with no size, is rust-js's error where it's
  stepped, as `$ptrRecast` was.
- hashbrown keeps its control bytes and its items in one allocation, and
  reaches items by stepping a byte pointer: bytes and items in one array,
  which this doesn't give.
