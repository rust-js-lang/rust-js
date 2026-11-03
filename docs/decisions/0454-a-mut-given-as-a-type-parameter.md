# 0454. A `&mut` given as a type parameter's value is a box

Status: Accepted; amended by [0566](0566-fuse-is-stds.md), which lifts the
refusal of a closure's parameter below: a function given for an `F:
Fn(&mut T)` takes generic code's box apart; and (2026-11-03) what's moved
out of an in-place cell is a copy apart (#527), and a `&mut` of an object
given for longer than the call is one (#528). Extends [0099](0099-mut-references.md) and
[0180](0180-generic-writers.md). Decided with the user.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same writes, to
the same place; and a compile error where generic code would have a box
and its caller the object (below).

## Context

A `&mut` to an object is the object in JS, and one to a number or text a
cell, `{ value }`. Generic code doesn't know which it has, so a generic
`&mut T` is given a box, copied back after the call (ADR 0099). But a
`&mut` given as a type parameter's value, `D` of `f(&mut v)`, was the
object, and an impl for `&mut T`, which has its `Self` in a box, read the
object's `value`: `impl<T: Get> Get for &mut T` called through `f<D:
Get>(&mut v)` threw. std's `impl Write for &mut W` and stable_deref_trait's
`StableDeref for &mut T`, which writeable and url take, were refused for
the same reason: generic code had no one form of a `&mut`.

## Decision

**A `&mut` given for a type parameter is a box, as one given for a
`&mut T` is: copied back after the call, and what's in it taken out where
the call gives it back. An impl for `&mut T` reads through it, and so do
std's `Deref` of a `&mut`, `(x) => x.value`, and its `fmt::Write`,
`$mutWriter(writer)`, the writer's own dictionary through the box.**

```rust
fn totalled<D: Total>(items: D) -> u32 {
    items.total()
}
println!("{}", totalled(&mut items));
```

```js
const items$1 = { value: items };
const value = totalled(items$1, refMutTTotal(vecTotal()));
$assign(items, items$1.value);
```

1. **One question, `mut_refs::boxed_in_call`**: of a type in a function's
   signature, through shared references, a `&mut` to its type parameter
   or its type parameter given a `&mut` to an object is a box. A
   parameter is given one, a result has its `value` taken out, and an
   impl for `&mut T`'s `&self`, called where its type is known, is given
   a box of the object, which nothing writes through.
2. **Elsewhere, refused**: a type parameter given a `&mut` to an object
   inside what generic code takes or gives, an `Option<D>`, a field, or a
   closure's parameter through `F: Fn(D)`, where generic code would have
   the box and its caller the object, as a nested `&mut T` was (ADR
   0099). A bound's own type, `D: Total`'s `D`, isn't a value.
3. **One writer through a box**, `$mutWriter`, of any writer: a `String`'s
   `$mutWriter($stringWriter)`, which was `$mutStringWriter`, a second
   form of the same job, and a crate's writer's, and a generic `W`'s.

## Why

- **One form of a `&mut` in generic code**: a box, whatever it points at,
  as the user decided; an impl for `&mut T` and generic code agree.
- **It's what Rust writes**: `impl Trait for &mut T` is how std and
  crates let a function take a writer, an iterator or a reader by value
  or by `&mut`.
- **It's tested**: a corpus case gives a `&mut` for a type parameter to
  a trait's impl for `&mut T`, through a dictionary and where its type is
  known, takes it back from a function, derefs it through std's `Deref`
  and writes through std's `Write`, to a crate's writer, to text and
  through a generic one, against native Rust; the refusals are in the
  limitations; a mutation of each part is caught.

## Consequences

- A `&mut` to an object given for a type parameter is boxed and copied
  back, `$assign(items, items$1.value)`, where the object was given before.
- `Option<D>` of `Some(&mut v)`, which generic code passed through as the
  object, is refused, listed in [the limitations](../guide/limitations.md).

## Amendment: a trait method's `&mut` result through a dictionary

A crate trait's `fn get(&mut self) -> &mut Self::Out`, called through a
dictionary, was read two ways: generic code that knew `Out = Vec<u32>`
took the result as the array, and generic code that didn't, as a box,
while the impl's entry gave the array. `push(&mut holder)` threw in JS
where Rust's pushed: a wrong answer. Case R
([0262](0262-when-rust-and-js-disagree.md)), now Rust's.

- **The trait's signature decides**, as it does for a function's: its
  `&mut Self::Out` is a box in generic code. A call through a dictionary
  takes `.value` of it where the caller knows the object, as a call of a
  generic function does, `generic_result_values`.
- **The entry gives that box**: an impl whose method gives the object,
  `&mut self.0` of a `Vec`, is `$cellOf(holderGet_get(arg0.value))`, a box
  a write of the whole changes in place, as `Cell::from_mut`'s is (ADR
  0153). A generic impl's own type parameter gives the box itself.
- std's `AsMut` and `DerefMut` give the box their signature says too.
- The `mut_assoc_through_dictionary` corpus case reads, pushes to and
  replaces through a crate trait's `&mut Self::Out` of a `Vec`, a number
  and a generic impl's `T`, against native Rust.

## Amendment: a `&mut` to a `&mut` of an object

rust_decimal's `impl AddAssign for &mut Decimal` has a `&mut self` that's a
`&mut &mut Decimal`, refused: a `&mut` to a value was a cell only where
that value was one, and a `&mut Decimal` is the object. A write of the
whole through it, `*r = &mut other`, makes it another, so it's a cell
too, `{ value: decimal }`, as a `&mut` to a number is. Not of an iterator
or a function, each the JS one (ADRs 0071, 0099), nor of a slice, which
may be a view of part of an array, refused as before.

Generic code holds a `Self` that's such a `&mut` in a box, as above, and
its `&mut self` in a cell of that box: each box a dictionary's entry is
given past what its method takes, a cell for each `&mut` it can't hold as
the object, is taken off, by `held_in_box`, the one question
`IntoIterator`'s entries ask too (ADR 0457). The
`mut_ref_to_object_impl` corpus case adds through such an impl where its
type is known and through generic code, against native Rust.

## Amendment: std's `&mut` of a type parameter's item is a handle

Where std gives a closure a `&mut` of an item, `retain_mut`'s,
`HashMap::retain`'s value, `extract_if`'s, or gives one back,
`select_nth_unstable`'s, `get_disjoint_mut`'s, an entry's `get_mut`, it
was a handle only of a value JS can't change in place, `is_boxable`: of
a type parameter's item, the item itself, though the closure made at the
call, a `&mut i32`'s, writes `n.value`, and a struct's reads its
handle's, `(x0) => f(x0.value)` (#131). `handles_mut` is the one answer:
of a value JS can't change in place, or of a type parameter, whose `&mut`
generic code reads through its `value`. Each such site in `slice.rs` and
`map.rs` asks it, held by the architecture test, and so does a guard of
a type parameter (ADR 0328). Held by `test/corpus/generic_retain_handles.rs`
and the mutation `type-parameter-mut-not-handle`.

## Amendment: a reborrow's cell given on

`impl<T: Trait + ?Sized> Trait for &mut T` forwards with `(**self).m()`,
a `&mut T` of the cell the `&mut T` is; a call of the crate's boxed it
again, a box of a copy, `{ value: self.value.value }`, so what `m` wrote
of a number or a `String` was lost (#167). The cell a place's `&mut` is
the `value` of, a call's, one made there, or one a place holds, is
`held_cell`'s answer, which a call's arguments and a value's handle both
ask: it's given on as it is. Held by `test/corpus/forwarding_mut_impl.rs`.

A `&mut &mut` of an object given for a type parameter of the crate's,
`drive(&mut &mut c)` through `impl<T: Trait + ?Sized> Trait for &mut T`,
needs generic code's box at each level, `{ value: { value: c } }`, where
the caller's `&mut` of an object is the object and the adaptation is one
level, `given_box`'s: it was a box short and threw (#167). It's refused
(`check_call_takes`), held by `test/corpus/mut_mut_object_to_generic.rs`;
a `&mut` of one, and a `&mut &mut` of a number, are given as they are.

## Amendment: a `PhantomData` holds no `&mut` (#516)

anyhow's `Mut<'a, T>`, a pointer with a `PhantomData<&'a mut T>` beside
it that only ties it to the borrow it was made of, was refused at every
call of a generic function that takes or gives one, "a `&mut
error::ErrorImpl` inside a generic function's parameters or result", with
the rest of anyhow behind it. Rule 2's walk, `nested_mut_object`, went
into every type a parameter or the result is made of, a `PhantomData`'s
own parameter too. But a `PhantomData` holds no value in JS or in Rust:
generic code has no box there, and its caller no object, so there's
nothing for them to disagree on. Case N
([0262](0262-when-rust-and-js-disagree.md)): no observable difference.

- **What a `PhantomData` is given isn't walked**: rule 2 refuses a
  `&mut` to an object only where a value holds one.
- `test/corpus/phantom_mut_marker.rs`, against native Rust: anyhow's
  `Mut` and `Ref` of a struct, made, moved, given back inside an `Option`
  and a tuple, and the struct changed while and after they're held.
- Mutations `phantom-mut-is-a-mut` (the walk goes in again) and
  `phantom-mut-skips-any-struct` (any struct's fields skipped as a
  `PhantomData`'s, which `test/diagnostics.test.ts`'s refusal of a struct
  holding a `&'a mut T` catches).
- anyhow is refused next at `{}` of a `dyn Error + Send + Sync` made
  where it's shown, an unrelated gap. regex-syntax's refusal, the same
  message of a `&mut ast::parse::Parser` in its `ParserI<'s, P>`, is
  rule 2's own: the field holds the value, generic code's box, where its
  caller would have the object.

## Amendment: what's moved out of an in-place cell is apart (#527)

`Cell::from_mut(&mut v).replace(vec![3])` gave `[3]` where Rust gives `[1,
2]`: an in-place cell, `$cellOf(v)`, `Cell::from_mut`'s of an object (ADR
0153) and a dictionary's `&mut Self::Out` (above), has the object as its
`value` and writes it in place, `$assign`, so `const old = cell.value;
cell.value = next;` read the object that the write then changed. `take`,
`swap` and generic code's `mem::replace`, `mem::take` and `mem::swap` of
such a cell were the same, and so was a `Copy` struct `get()` read before a
`set`. A wrong answer. Case R ([0262](0262-when-rust-and-js-disagree.md)),
now Rust's.

- **What's moved out of a cell or a box is apart from what's written
  next**, `mut_refs::moved_out`, the one answer: of an object or a type
  parameter's, `$apart(old)`, the copy an object's box holds where it's
  given (ADR 0074). `Cell`'s `replace` and `take` ask it; and generic
  code's `mem::replace`, `mem::take` and `mem::swap` through a reference,
  which may be a box, an in-place cell too: `const old = $apart(r.value);
  r.value = v;`. Never in place, and read as before: a number's or a
  string's cell, a `RefCell`, a thread-local's cell, a place of the body's
  own, `&mut self.0` or a `Vec`'s item, `&mut heap[0]` (`borrows_own_place`),
  and an object's `&mut`,
  `$take(r, v)` (ADR 0147), whose copy `$apart` now makes too.
- **`$cellSwap` moves each out apart** before either is written, as either
  may be in place.
- **`Cell::from_mut` replaces whole** the object its `&mut` is to
  (`replaces_whole`), as `mem::replace` does, so a `Copy` value read from
  its cell is copied where it's changed in place after.
- A replace whose old value nothing reads is the write alone, as before:
  tidy drops `$apart(dest.value)` as it dropped `dest.value`.
- **What a `&mut` given to `mem::replace` is reached through is named
  once**, `const item = h.get();`: `mem::replace(h.get(), v)` called
  `h.get()` for its read and again for its write, and store_get_mut's
  `insert_at` called `lm_get_mut` twice, a wrong answer of its own where
  the call does something.
- An object's box given to generic code still holds a copy apart (ADR
  0074, `may_move_out`), so what generic code moves out of it is copied
  twice: once where it's given and once where it's moved. Kept: the box's
  copy also covers what generic code moves out other than through these
  calls, which `moved_out` doesn't see.
- Held by `test/corpus/in_place_cell_replaced.rs` and
  `test/corpus/in_place_cell_moved_out.rs`, against native Rust: `replace`,
  `take` and `swap` of `Cell::from_mut`'s cells of a `Vec`, a `String` and
  a struct, of a parameter's and a local's, through generic code and a
  dictionary's `&mut Self::Out`, and a `Copy` struct's `get()` before a
  `set`, and a `get()` that prints, reached once. Mutations
  `moved-out-shared`, `moved-out-of-object-shared`,
  `generic-moved-out-shared`, `cell-swap-shared`, `from-mut-not-whole`,
  `replaced-holder-reached-twice`, `checked-index-named-for-replace`,
  `own-place-moved-out-apart`, `indexed-item-moved-out-apart`,
  `ref-cell-moved-out-apart`, `unread-apart-kept` and `apart-acts`.

## Amendment: a `&mut` of an object given for longer than the call is an in-place cell (#528)

`fn set_get<'a, T>(a: &'a mut T, v: T) -> &'a mut T { *a = v; a }`, given
`&mut v` of a `Vec`, wrote nothing the caller saw where `v` was a `&mut
Vec` parameter, and threw "assignment to constant `v2`" where it was a
local: a `&mut` whose borrow outlives the call (`borrow_outlives_call`,
ADR 0099) was a handle on the variable that names it here, `set value(x)
{ v = x; }`, which rebinds a parameter's copy of the caller's object, or a
`const`, where a write through a `&mut` to an object changes the object
in place (ADR 0147). A wrong answer. Case R
([0262](0262-when-rust-and-js-disagree.md)), now Rust's.

- **Of an object, it's an in-place cell**, `$cellOf(v)`,
  `mut_refs::in_place_cell`, the one form of a box whose write of the
  whole changes the object, `Cell::from_mut`'s and a dictionary's `&mut
  Self::Out`'s (above). A number's or a string's stays a handle on its
  place, which JS can't change in place.
- What generic code moves out of it is a copy apart (`moved_out`, above).
- The objects such a call is given read as written, `const c = { n: 1 }`,
  `pick(true, $cellOf(c), $cellOf(d))`, where each was a `let` a handle of
  nine lines rebinds: `generic_mut_ref_kept`, `mut_assoc_through_dictionary`
  and `phantom_mut_marker` lost 119 lines.
- Held by `test/corpus/generic_mut_written_whole.rs`, against native Rust:
  `set_get` of a `Vec` given by a parameter and by a local, its result
  written through after, of a `String`, a struct and a number, and through
  a generic function that gives it on, and a `Copy` struct read before.
  Mutation `outliving-object-handle`.

## Amendment: regex-syntax's `ParserI` stays refused (#516, the user's decision)

Decided by the user (2026-11-04): #516's unfixed half, regex-syntax's
`ParserI<'s, P> { parser: P }` given a `&mut ast::parse::Parser`, stays
refused for now, a documented limitation (the guide's limitations page,
a `&mut` to an object given for a type parameter inside what generic
code takes or gives). Its field holds generic code's box where its
caller would hold the object, rule 2's own case (the `PhantomData`
amendment above). Revisit it once #527's `moved_out` owner (above) has
settled.
