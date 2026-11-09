# Draft: a raw pointer of a type parameter is a handle on its place

Status: Draft, for the user's decision (#1140). Would amend
[0454](../0454-a-mut-given-as-a-type-parameter.md), generic code's box of
a `&mut`, and [0562](../0562-a-pointer-is-a-place-and-an-address.md), a
pointer is a place and an address, and would extend
[0099](../0099-mut-references.md)'s rule of what a caller gives a `&mut`.
The work so far, which doesn't land as it is, is on the local branch
`wip/1140-generic-raw-pointers`.

Case ([0262](../0262-when-rust-and-js-disagree.md)): N where it's
decided below; a compile error, as today, where a pointer could leave a
call some other way.

## Context

Crates whose minimum Rust predates `ptr::from_mut` write its polyfill:

```rust
fn ptr_from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

unsafe fn slice_as_uninit_mut<T>(slice: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = ptr_from_mut::<[T]>(slice) as *mut [MaybeUninit<T>];
    unsafe { &mut *ptr }
}
```

getrandom 0.3's `util.rs` is one such crate, so its `fill` is types alone. rust-js refuses
`&raw mut *r` of a `&mut` JS holds in a box, "a raw pointer to what a `&mut`
refers to, which JS holds in a box of a copy". Generic code's `&mut T` is
always such a box (`MutRefForm::GenericBox`). Two things make lifting the
refusal more than a refusal's removal. Both were seen running the
polyfill on the branch above.

1. **A caller's box is a copy.** `bump(&mut n)` is `const r = { value: n };
   bump(r); n = r.value;`, and a field's is the same, copied back in a
   `finally`. A pointer the callee makes of `r.value` is of the box. Written
   after the call, `let p = ptr_from_mut(&mut n); *p += 1;`, it writes the
   dead box, and `n` stays 5 where native Rust's is 6.
2. **What a raw pointer is depends on its type.** A pointer to a number is a
   handle, `$ptr(cell, "value", 0)`. A slice's, `*mut [U]`, is the array
   itself (ADR 0562). Generic code is compiled once, so `ptrFromMut`
   returns a handle whatever `T` is, and `slice_as_uninit_mut` used it as
   the array: `uninit[1] = 20` wrote the handle's property, and `bytes`
   stayed `[1, 2, 3]` where native Rust's is `[1, 20, 30]`. A shared
   `ptr_from_ref(&n)` of a number is given the number, a copy, and
   `$ptrTo(n)` throws. std's own `ptr::from_mut`, lowered as `FromRef`,
   gives a cell where the polyfill gives a handle: two forms of one
   pointer.

## Proposal

**In generic code, a raw pointer whose pointee is a type parameter, or
holds one, is one form whatever the parameter is: a handle on the place,
whose `value` is the referent.** Callers adapt at the boundary, as ADR
0454 does for generic code's `&mut` box.

- **Made in generic code** of a `&mut T`, `&raw mut *r`, the polyfill's `r`,
  or std's `ptr::from_mut(r)`: a handle on the box's place,
  `$ptr(r, "value", 0)`. Of a `&T`, `ptr::from_ref`: the same, of what its
  caller gives as a place (below).
- **Given into generic code** where a `*mut T` goes: a pointer the caller
  has in its own form is wrapped. A number's handle is given as it is; an
  object's or a slice's, the array or object itself, as `$cellOf(it)`-like
  handle whose `value` is it, `$ptrOf(object)`.
- **Given back by generic code**, a result or a field of type `*mut T`:
  read in the caller's form. Of an instantiation whose pointer is the
  object itself, a slice's or an object's, `.value`. Of one whose pointer
  is a handle, a number's, as it is. This is `generic_result_values`' rule
  for a pointer, beside its box's.
- **Passing a `&mut` or a `&T` where a pointer can leave the call.** A
  function whose signature has a raw pointer in its result or another
  parameter is given the place itself, never a box of a copy:
  - a whole local of a number is a cell, `const n = { value: 5 }`, given as
    it is;
  - a field's or an item's place is a handle, `$ptr(s, "n")`, whose `value`
    reads and writes it in place;
  - a `&T` of a number is given as such a handle too, read through.
  This is of the signature alone (`lets_pointers_out`), the same at a
  direct call, a dictionary's or a `dyn`'s, a fn pointer's, a closure's of
  `F: FnMut(&mut T) -> *mut T`, and a library's function's, read from
  rustc's metadata. A function whose signature lets no pointer out keeps
  today's copy in and copy back, its JS byte for byte the same.
- **Escaping some other way**, into a `static`: refused, as today.

| Pointee in generic code | Instantiation | Given in | Given back |
|---|---|---|---|
| `T` | a number, `u32` | its handle, as it is | as it is |
| `T` | an object, `Vec<String>` | `$ptrOf(object)` | `.value` |
| `[T]` | a slice, `[u8]` | `$ptrOf(array)` | `.value` |
| `T` | a type parameter of the caller's own | as it is, already this form | as it is |

How it relates to ADR 0454: there, generic code's `&mut T` is a box,
`{ value }`, which a caller of an object gives an in-place cell of it,
`$cellOf`. Here generic code's `*mut T` is a handle on a place, whose
`value` is the referent, which a caller of an object gives a handle whose
`value` is the object. Both are "a cell of the referent". The pointer's
also has a place an address is asked of (ADR 0562), as the box needn't.

## What holds it

Corpus cases against native Rust:
- the polyfill of a slice, `slice_as_uninit_mut` and
  `slice_assume_init_mut`, writes through to the array;
- a number's pointer written after the call (`n` 6, not 5);
- an object's pointer, `(*q).push(..)`;
- `ptr_from_ref` of a number, read;
- std's `ptr::from_mut`/`from_ref` in generic code;
- each call shape: direct, through a dictionary, a `dyn`, a fn pointer, a
  closure `F: FnMut(&mut T) -> *mut T`, and a library's function.

Every corpus case's JS whose functions let no pointer out unchanged.
Mutations of each adaptation, given in and given back. The real
getrandom 0.3.4 with `wasm_js` through the patch (ADR 1112), filling
bytes in Node.

## Open questions

- Does a `*const T` of a `&T` of a number given to generic code need the
  place, or is a copy enough? Reading through it is the same. An
  address, `p.addr()`, and comparing it with another, `ptr::eq`, are not.
  The proposal gives the place.
- A struct of generic code holding a `*mut T`, a field: adapted where it
  crosses the boundary, as a result's is, or refused at first.
