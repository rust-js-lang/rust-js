# 1114. A reference a raw pointer may leave with is its place's pointer

Status: Accepted, the user's decision (#1143, 2026-11-10: "the place
handle"), the first fact landed under [1115](1115-what-generic-code-needs-of-its-type-parameters-has-one-catalogue.md)'s
catalogue; what's left is listed under Consequences as still refused. Amends
[0454](0454-a-mut-given-as-a-type-parameter.md), generic code's box of a
`&mut`, [0099](0099-mut-references.md), what a caller gives a `&mut`, and
[0562](0562-a-pointer-is-a-place-and-an-address.md), a pointer is a place
and an address. Replaces the draft `generic-raw-pointers`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same reads and
writes of the same place, the same pointers equal; a compile error where
a pointer would be of a value JS holds as itself.

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

getrandom 0.3's `util.rs` is one (#1140). rust-js refused `&raw mut *r` of
a `&mut` JS holds in a box of a copy. Lifting the refusal alone gives wrong
answers, both seen running the polyfill:

1. **A caller's box is a copy.** `bump(&mut n)` is `const r = { value: n };
   bump(r); n = r.value;`. A pointer the callee makes of `r.value` is of the
   box: written after the call, `let p = ptr_from_mut(&mut n); *p += 1;`, it
   writes the dead box, and `n` stays 5 where native Rust's is 6.
2. **A raw pointer's JS depends on its pointee.** A number's is a handle,
   `$ptr(cell, "value", 0)`; an object's a handle on it, `$ptrTo(v)`; a
   slice's is the array itself, and a pair `dyn`'s the pair (ADR 0562).
   Generic code is compiled once, so `ptrFromMut` returned a handle
   whatever `T` was, and `slice_as_uninit_mut` used it as the array:
   `uninit[1] = 20` wrote the handle's property, and `bytes` stayed
   `[1, 2, 3]` where native Rust's is `[1, 20, 30]`.

## Decision

**A raw pointer has no lifetime, so a function whose signature has one in
its result or in another parameter may let a reference it's given leave
the call as a pointer: such a function is given, for each reference
parameter, its place's pointer, never a box of a copy; and in it, `&raw
mut *r` of the parameter is the parameter.** This is of the signature
alone (`lets_pointers_out`), so a direct call, a dictionary's, a function
pointer's, a closure's or an `F: FnMut(&mut T) -> *mut T`'s, by its
arguments' tuple and its result, and a library's function's agree; what a
call calls is `call_callee`'s, an impl's method resolved, as its arguments
are given, so a `dyn`'s receiver, its pair, is given as it is.

**In generic code, a raw pointer whose pointee is a type parameter is a
handle on its place, whose `value` is the referent, whatever the
parameter is.** A caller adapts at the boundary only where its own form
isn't a handle, `form_given` and `form_back` (ADR 1115's `Form::Handle`):

| Pointee in generic code | Instantiation | Caller's pointer | Given in | Given back |
|---|---|---|---|---|
| `T` | a number, `u32` | `$ptr(n, "value", 0)` | as it is | as it is |
| `T` | an object, `Vec<String>` | `$ptrTo(v)` | as it is | as it is |
| `T` | a slice, `[u8]`, or a pair `dyn` | the array, the pair | `$ptrTo(array)` | `.value` |
| `T` | a JS object | the object | `$ptrTo(object)` | `.value` |
| `T` | the caller's own type parameter | a handle already | as it is | as it is |

A reference given to a function that lets pointers out is given as the
pointer `&raw mut *r` would make of it, in the callee's form:

- a whole local of a number is a cell, `const n = { value: 5 }`, given as
  `$ptr(n, "value", 0)`, the same handle `&raw mut n` is, so `ptr::eq` of
  the two is `true`, as Rust's;
- a field's or an item's, `$ptr(s, "n", 4)`, `$ptr(items, i)`;
- an object's, `$ptrTo(v)`, whose `value` is the object and whose write
  assigns it in place, as an in-place cell does (ADR 0454);
- a slice's, to generic code, `$ptrTo(array)`; to a function of slices,
  the array, which a slice's `&mut` already is.

`&T` is given the same way: a shared reference to a number, which JS holds
as the number, is given as its place's pointer, read through, where the
callee lets pointers out. A function that lets no pointer out is given
what it was, its JS byte for byte the same.

A raw pointer in another parameter counts where the callee may store
one there, behind a reference or a pointer, `out: &mut *mut T`; one given
by value, `note(p, count)`'s `p`, takes nothing out. Only a call rust-js
compiles is given so, the crate's function or a compiled trait's method:
std's, which their own lowering reads, `MaybeUninit::as_mut_ptr(&mut m)`,
are given what they were.

A generic function's instance as a value takes generic code's own forms,
a raw pointer's handle as a `&mut T`'s box, as ADR 0454's
`takes_box_itself` gives it (#975): generic code that calls it as an `F`
gives them. So as a `fn` pointer of an instance whose forms differ, an
object's or a slice's `&mut T`, a slice's or a JS object's raw pointer,
whose concrete callers give their own, it's refused (#1187), where it
threw before.

### Why this form

- **It's the place, so it's exact**: reads and writes through the pointer
  reach the caller's variable, field, item or object, during and after the
  call; two pointers to one place are one handle, `===`.
- **It's one form in generic code**, as ADR 0454's box is: a handle whose
  `value` is the referent, which generic code reads and writes without
  knowing `T`. A caller adapts only the instantiations whose pointer isn't
  a handle, at the boundary, where it knows `T`.
- **It costs nothing elsewhere**: a function without a raw pointer in its
  signature is compiled and called as before.

## Alternatives

- **(A) The signature rule alone**, without the generic form: slices'
  writes were lost (`bytes` `[1, 2, 3]`), the experiment on the branch
  `wip/1140-generic-raw-pointers`. Rejected.
- **A pointer's form by its pointee everywhere, generic code dispatching
  at run time**: every generic pointer read would test what it holds.
  Rejected: the boundary knows `T`, so generic code needn't.
- **Refusing the polyfill**: getrandom 0.3, and every MSRV-bound crate
  writing it, stays refused. Rejected by the user.

## Consequences

- Delivered in steps, each holding its corpus cases against native Rust,
  per call shape (direct, a dictionary's, a `dyn`'s, a function pointer's,
  a closure's, a library's) and per instantiation (a number, an object, a
  slice, the caller's own type parameter), with mutations of each
  adaptation, given in and given back, and the whole net.
- A struct of generic code holding a `*mut T`, a field, crossing the
  boundary, of a slice's instance: not adapted, as a result's is.
- A pointer of a reference that escapes another way, into a `static`:
  refused, as before.
- The real getrandom 0.3 with `wasm_js`, through ADR 1112's patch, fills
  bytes in Node once the steps land (#1140).

## Delivery

**Step 1 (2026-11-10).** The signature rule, `lets_pointers_out` and
`given_its_pointer` (`mut_refs.rs`), a closure's and a function pointer's
by their signature (`sig_given_its_pointer`); the body fact
`Pointed::lent_as_pointers` (`pointers.rs`), each reference made once and
given to such a call, followed through a reborrow to the whole local it
makes a cell; such a reference made as `&raw mut` of its place
(`mir/assign.rs`); `&raw mut *r` of a parameter given its pointer, the
parameter (`given_its_pointer_here`); the generic form adapted at a call's
arguments and result and where a generic function is a function pointer
(`generic_facts.rs`); and an instance whose forms differ refused as a
`fn` pointer (`reified_takes_generic_forms`, #1187): `view::<Vec<u8>>` as a `fn(&mut Vec<u8>) -> &Vec<u8>` read
`r.value` of the vector, a `TypeError`.

Held by `reference_given_as_its_pointer.rs`, each instantiation (a number,
an object, a field, an item, a slice, the caller's own type parameter)
and call shape (direct, a dictionary's, an impl's own, a `dyn`'s, a
function pointer's, a closure's, an `F: FnMut`'s, through another
parameter, a pointer given by value), each pointer `ptr::eq` to `&raw mut`
of its place;
`generic_fn_item_takes_generic_forms.rs`; the crate pair
`library_lets_a_pointer_out`; `shared_number_given_to_let_a_pointer_out_refused.rs`;
`generic_fn_pointer_of_an_objects_mut_refused.rs`,
`generic_fn_pointer_of_a_slices_pointer_refused.rs`; `test/webapi-objects.test.ts`'
JS object's instance as a `fn`, refused; all against native Rust and with
`METAMORPHIC=all`; and twenty
mutations, `lower/mut_refs.ts`, `lower/pointers.ts`, `lower/mir/assign.ts`,
`lower/mir/rvalues.ts`, `lower/generic_facts.ts` and `lower.ts`.

## Amendment: std's `ptr::from_mut` and `ptr::from_ref`, the polyfill's one path (2026-11-10)

std's own `ptr::from_mut(r)` and `ptr::from_ref(r)` were refused, odd beside
the polyfill compiling: made of a reference as `FromRef` makes one, their
pointer was no handle `&raw mut` of the place is, `ptr::eq` of them `false`
where Rust's is `true`. **They're the polyfill's one path
(`PointerOp::FromPlace`): the reference given to one, made where it's
given, `&mut k` or `&k`, a shared one's too, is lent as its place's pointer
(`Pointed::lent_calls`), and the call is that pointer,
`std::ptr::from_mut(&mut k)` `$ptr(k, "value", 0)`, `&raw mut k`'s handle;
one given otherwise, a parameter's `r` as it is, as `FromRef`.** Held by
`reference_given_as_its_pointer.rs`, std's in generic code and of a local,
a shared number's read and `ptr::eq`, and the mutations
`from-place-lent-reread` and `std-from-place-not-lent`.

## Amendment: a generic function's value takes generic code's forms (2026-11-10)

Step 1 adapted a raw pointer of a type parameter where a generic function
was a value, `(arg0) => measured($ptrTo(arg0), u8SliceLen).value`, as its
concrete callers give the array. But generic code given the function as
an `F: Fn(*mut T) -> *mut T` gives it its handle, which the value wrapped
again: `apply_ptr(p, measured::<[u8]>)` showed `undefined` where native
Rust shows `3`, a wrong answer, a fn item's and a `fn` pointer's alike.
**A generic function's value takes generic code's forms, as it takes its
`&mut T`'s box (#975); a `fn` pointer of an instance whose forms differ is
refused (`reified_takes_generic_forms`, #1187).** Held by
`generic_fn_item_takes_generic_forms.rs` against native Rust,
`generic_fn_pointer_of_a_slices_pointer_refused.rs`, and the mutation
`reified-slice-pointer-allowed`.

## Amendment: a generic function's `fn` pointer is a function of its own (2026-11-10, #1187)

A generic function's instance whose forms differ from generic code's, an
object's or a slice's `&mut T`, a slice's or a JS object's raw pointer, was
refused as a `fn` pointer, as its value takes generic code's forms and the
pointer's concrete callers give their own. **Its `fn` pointer is a
function of its own (`reified_pointer`, `generic_facts.rs`): it takes its
type's forms, as its type's other functions do, and gives the instance
generic code's, `Form::Handle`'s `$ptrTo(arg0)` and `Form::Box`'s
`$cellOf(arg0)`, an in-place cell (ADR 0454), and what it gives back in
its type's, `.value`; one for the instance, its module's `const`
(`Kind::FnPointer`), so two pointers to it are one, as its value is (ADR
0395).** Generic code given the pointer as an `F` gives it its type's forms,
as for any function of that type (`unboxed_by_function`). A library's
function's value, the arrow of its call, takes generic code's forms of
both, its box too, as generic code gives it them (#975). Held by
`generic_fn_pointer_takes_its_types_forms.rs` against native Rust with
`METAMORPHIC=all`, which replaces the two refusals' cases, the crate pair
`library_generic_fn_item_forms`, `test/webapi-objects.test.ts`' JS object's,
and the mutations `reified-instance-as-its-value`,
`reified-slice-pointer-as-its-value`, `reified-object-mut-unboxed`,
`fn-pointer-keyed-as-its-value` and `box-form-unasked`.

