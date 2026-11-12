# 0454. A `&mut` given as a type parameter's value is a box

Status: Accepted; amended by [1115](1115-what-generic-code-needs-of-its-type-parameters-has-one-catalogue.md), 2026-11-10: the box is `Form::Handle`, adapted where a generic function is a function pointer too, and by [0566](0566-fuse-is-stds.md), which lifts the
refusal of a closure's parameter below: a function given for an `F:
Fn(&mut T)` takes generic code's box apart; and (2026-11-03) what's moved
out of an in-place cell is a copy apart (#527), and a `&mut` of an object
given for longer than the call is one (#528); and (2026-11-04) generic
code's box of an object holds the object itself (#606), and `Cell::from_mut` of
a type parameter's `&mut T` is that box (#614), and what a bound trait's
methods are given is walked, a generic function's (#682) and a generic
impl's (#685), past what its entry adapts within an `Option` or a tuple
(#686); and a copied default calls through its impl's entry where the entry
adapts more than a call does (#690), reading a generic impl's dictionary once
(#691). Extends [0099](0099-mut-references.md) and
[0180](0180-generic-writers.md). Decided with the user; amended by [1086](1086-a-mut-refs-js-form-is-mut-ref-forms.md): a type parameter's box is `MutRefForm::GenericBox`, asked of `mut_ref_form`.

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

Superseded (2026-11-12, #1405): the field holds generic code's box
wherever the struct is, decided for the user; see the amendment below.

## Amendment: generic code's box of an object holds the object (#606)

The #527 amendment kept the copy an object's box held where it's given,
`{ value: $apart(stack) }` (`may_move_out`), beside the copy `moved_out`
makes where generic code moves the value out, "for what generic code moves
out other than through these calls". Two owners of one rule, and the
Nightly of 3c71ee8f9 found the first's mutation, `moved-out-of-box-unseen`,
surviving: with no copy where it's given, every corpus case ran as native
Rust does, and only six files' JS changed, each losing a `$apart`.

Safe Rust moves a `T` out of a `&mut T` only through std's calls, which
are `moved_out`'s: `mem::replace`, `mem::take`, `mem::swap` and `Cell`'s
`replace` and `take`, through any reference the `&mut` reaches generic
code by. `test/corpus/generic_mut_moved_out_other_ways.rs`, new, runs the
other ways against native Rust: through a struct that holds the `&mut`, a
closure that captures it, `mem::take`, `mem::swap` with a local, into a
`Vec` it's given, and through a trait's impl of a concrete type, of a
struct and of a `Vec`. Each prints what Rust does with the box holding the
object itself.

So **the box holds the object, `{ value: stack }`**, and `moved_out` is the
one copy. `may_move_out` and its mutation are gone. Found beside it, and
filed as #614 (P0): `Cell::from_mut` of a type parameter's `&mut T` makes
an in-place cell of generic code's box, where the box is that cell. The
same JS before and after this change.

## Amendment: `Cell::from_mut` of a type parameter's `&mut T` is its box (#614)

`fn through_cell<T>(r: &mut T, v: T) -> T { Cell::from_mut(r).replace(v) }`
threw `TypeError: Cannot read properties of undefined` where Rust gives
the old value: `Cell::from_mut` asked `is_boxable(T)`, false of a type
parameter, so it made an in-place cell of generic code's box,
`$cellOf(r)`, whose `value` is the box, not the value. A wrong answer,
found beside #606. Case R ([0262](0262-when-rust-and-js-disagree.md)), now
Rust's.

What a `&mut` to a `T` is has one owner, `is_cell_pointee`: a cell of a
number or a text, and generic code's box of a type parameter's
(`is_generic_boxed`). `Cell::from_mut` and its inverse, `Cell::get_mut`,
ask it, not `is_boxable`: of a cell pointee, the `&mut` is the cell
already, `r` itself; of an object, `$cellOf(v)`, as before. `get_mut` of a
type parameter's `Cell<T>` gave `cell.value`, the value, where generic code
reads a `&mut T` as a box.

Held by `test/corpus/cell_from_mut_generic.rs` against native Rust:
generic `replace` of a `Vec`, a struct, a number and a `String`, `get` and
`set` of a `Copy` struct and a number, `take`, `get_mut` given to
`mem::replace` of a `Vec` and a number, and `as_slice_of_cells` of a type
parameter's items and of numbers, swapped and replaced, beside the
concrete forms. Mutations `cell-from-mut-of-a-param-wrapped` and
`cell-get-mut-of-a-param-unwrapped`.

## Amendment: what a bound trait's methods are given is walked (#682)

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't
be preserved yet, so it's a compile error, as rule 2's other cases are.

Generic code calls a method of a trait its type parameter is bound by,
`S: Pick`, through the trait's dictionary, and gives it what it takes as
generic code has it. The impl's entry turns a parameter's own `&mut` into
what its impl takes, and the result's into what generic code takes (ADR
1091, amended #679), but it doesn't change a `&mut` inside a parameter.
So `s.pick(Some(&mut w))` of `fn pick(&mut self, other: Option<&mut
Self>)` gave a `Clicks` impl generic code's handle on `w`, where it takes
the object, and its writes went to the handle: native Rust printed
`Clicks { n: 51 }`, rust-js `Clicks { n: 1 }`. That was a wrong answer.

Rule 2's walk, `nested_mut_object`, now walks what each method of a bound
trait of the crate's takes and gives, with the trait's `Self` as the
bound gives it, past each parameter's and the result's own `&mut`, just
as it walks a function's own signature. A call that gives a type
parameter an object is refused there, with the message rule 2 gives;
of a number's impl, whose box generic code's is, it isn't.

- `test/diagnostics.test.ts`: the refusal of an `Option<&mut Self>` and a
  `(&mut Self, i32)` a bound trait's method takes.
- `test/corpus/nested_mut_self_of_a_number.rs`, against native Rust: both
  shapes through a `u32`'s dictionary.
- `docs/guide/limitations.md` lists it.
- Mutations `dispatched-nested-mut-unwalked` (the methods unwalked) and
  `dispatched-nested-mut-of-own-receiver` (a method's own `&mut self`
  walked too, which refuses every generic call of an object's `&mut self`
  method).

## Amendment: what a generic impl's bound trait's methods are given is walked where its dictionary is made (#685)

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't
be preserved yet, so it's a compile error, as #682's is.

#682's walk asked a generic function's own bounds, and a call of an impl's
method where its type is known, `h.wrap()`, whose bounds are its impl's
too. But a generic impl's dictionary, made for generic code or a `dyn`,
`holderWrap(clicksPick)` of `impl<T: Pick> Wrap for Holder<T>` and a
`Holder<Clicks>`, had none: its `wrap`, compiled once for every `T`, calls
`T::pick(.., Some(&mut self.b))` through `T`'s dictionary with generic
code's handle on `self.b`, and `Clicks`'s entry gave its impl that handle,
whose writes were lost. Native Rust printed `Clicks { n: 51 }` for `b`,
rust-js `Clicks { n: 1 }`: a wrong answer.

- **The impl's own bounds are walked where its dictionary is made**,
  `impl_nested_mut_object`, by what a function's call asks of its own:
  `dispatched_takes` gives each, past a parameter's and the result's own
  `&mut`, and `nested_mut_within` finds a `&mut` to an object of the
  impl's arguments there. The crate impl's dictionary is refused, "a
  `&mut C` inside what a generic impl gives a method of a trait it's bound
  by". Of a number's impl, whose box generic code's is, it isn't.
- `test/diagnostics.test.ts`: the refusal of such a dictionary given to
  generic code and made a `dyn`.
- `test/corpus/nested_mut_self_through_a_generic_impl.rs`, against native
  Rust: a number's, through generic code, where its type is known and as a
  `dyn`.
- `docs/guide/limitations.md` lists it.
- Mutation `impl-nested-mut-unchecked` (`traits/dictionaries.ts`); the
  #682 mutations name the walk as `dispatched_takes` has it.

## Amendment: a dictionary's entry adapts a `&mut` within an `Option` or a tuple (#686)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so it
keeps Rust's behavior. A refusal (#682, #685) is lifted.

#682 and #685 refused a call that gave a bound trait's method a `&mut` to
an object within a parameter, `pick(&mut self, other: Option<&mut Self>)`,
because the impl's entry gave its impl generic code's box. Now the entry
converts that `&mut` the same way it converts a parameter's own, both
ways:

```js
export const clicksPick = {
  pick(arg0, arg1) {
    return clicksPick_pick(arg0.value, arg1?.value);
  },
  first(arg0, arg1, arg2) {
    const given = clicksPick_first(arg0.value, arg1.value, arg2);
    return given != null ? $cellOf(given) : undefined;
  },
};
```

- **One shape, `mut_refs::Within`**, made by `within_shape`. A `&mut`
  that's generic code's box, `GenericBox` in generic code's terms, is
  `Mut` of the form a `&mut` to the impl's type takes. Its tuples and its
  `Option`s are read field by field, but only an `Option` whose `Some`
  boxes neither side (`boxed_payload`) and that isn't an `Option` of an
  `Option`. Anything else is `AsItIs`. It applies past a parameter's or
  the result's shared references (`adapted_part`). The parameter's or the
  result's own `&mut` is converted as before.
- **What the entry does**, `Within::adapted`: toward the impl, a cell's
  form is kept as it is, and any other form gets the box's `value` (the
  object, #606), `arg1?.value` of an `Option`. Back, any other form gets
  an in-place cell, `$cellOf`, as a result's own `&mut` does. A tuple is
  rebuilt only where a field changes, `[arg1[0].value, arg1[1]]`.
- **What's refused is what's `AsItIs`**: #682's walk (`dispatched_takes`)
  and #685's (`impl_nested_mut_object`) go past what `within` adapts and
  walk only the rest, so a `Vec<&mut Self>` is still refused at the call
  and where a generic impl's dictionary is made.
- **The declaration says it** (ADR 1091): `traits::method_dictionary`
  asks `within_shape` for the same shape, so `pick(self: { value: Self },
  other?: { value: Self } | null | undefined): boolean` is declared. A
  `Vec<&mut Self>` is still left out.
- A copied default's own call of such a method doesn't go through the
  entry, which is #690, a wrong answer of its own.
- `test/corpus/nested_mut_self_through_a_dictionary.rs`, against native
  Rust: an `Option`, a tuple, an `Option` of a tuple, and a result's
  `Option` and tuple, each through the dictionary of an object's impl, a
  number's, and a generic impl's of each, with a reborrow given within.
  `test/corpus/nested_mut_self_through_a_generic_impl.rs` adds #685's
  object case. `test/declarations-conformance.test.ts` calls the case's
  functions from TypeScript with each impl's dictionary as declared.
  `test/diagnostics.test.ts` refuses a `Vec<&mut Self>` and a `&mut
  Option<&mut Self>`.
- Mutations `within-option-unadapted`, `within-mut-given-generic-box`,
  `within-mut-given-back-as-the-object` and `dispatched-adapted-walked`
  (`mut_refs.ts`), `entry-nested-mut-unadapted` (`traits/impls.ts`), and
  `declared-nested-mut-left-out` (`traits.ts`). The #682 mutations point
  at the `Vec` refusals.

## Amendment: a copied default calls through its impl's entry where the entry adapts more than a call does (#690)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so it
keeps Rust's behavior.

A trait's default method is copied into each impl (ADR 0049). Its body
stays in the trait's terms, where `&mut self` and every `&mut Self` it holds
are generic code's box. Its calls of its own trait's methods on `Self`
resolve to the impl's function, so they skipped the dictionary's entry,
which converts each form. A direct call converts only each argument's own
`&mut`, as the callee's parameter says (`boxed_args`). It got the rest
wrong. It read a `-> &mut Self` the impl gave as the object as if it were
a box: `*r = Self::fresh()` wrote `r.value`, giving native `n: 99` and
JS `n: 1`. The same happened with `-> Option<&mut Self>`. It also gave the
impl generic code's box inside an `Option<&mut Self>` argument, so the
impl's writes went to the box. That was a wrong answer that nothing
refused.

- **One answer of what an entry adapts**, `EntryForms`, made by
  `entry_forms` in `traits/impls.rs`. It holds each parameter's boxes
  taken off, `Self` held in a box, what's within each parameter
  (`Within`, #686), and the result's own `&mut` and what's within it.
  The entry is built from it.
- **A copied default calls through the entry** where the entry converts
  more than a direct call does (`EntryForms::beyond_a_call`): a `Self`
  held in a box, a `&mut` within what it takes, or anything it gives
  back. `through_entry` is that one answer. `impl_method` asks it, so the
  call's arguments and result are generic code's (`boxed_args`,
  `generic_result_values`), and so does `trait_call`'s resolved call, so
  the call is the dictionary's: `const r = clicksTouch.me(self);`, whose
  `$cellOf` a write of the whole changes in place.
- Where only each argument's own `&mut` differs, the copied default calls
  the impl's method itself, as before, `clicksTouch_touch(self.value,
  other.value)`. A call converts that itself, as it does where the type is
  known. Routing it through the entry made seven corpus cases box a
  by-value `self` and copy it back, `const self$1 = { value: self };
  xCh.set(self$1, 5); self = self$1.value;`, with the same answer but
  worse JS, so it was rejected.
- A generic impl's copied default calls its own dictionary, which
  `$traitImpl` caches, read once at its top where it calls it twice or
  more, `const HolderTouch = holderTouch(TTouch);` then
  `HolderTouch.me(self)`, as any body reads a dictionary it writes twice
  ([0979](0979-a-dictionary-a-body-writes-twice-is-declared-once.md),
  amended, #691).
- `test/corpus/copied_default_through_its_entry.rs`, against native Rust:
  a `-> &mut Self` and a `-> Option<&mut Self>` given back and written
  whole, an `Option<&mut Self>` given a reborrow and then the `&mut`
  itself, a `&mut Self` given as its own, and, with no `&mut self`, a
  `&mut Self` and an `Option<&mut Self>` given back. Each runs for a
  struct's impl, a number's and a generic impl's, called where the type
  is known and through generic code.
- Mutations `through-entry-never`, `through-entry-result-cell-unasked`,
  `through-entry-result-within-unasked` and `through-entry-within-unasked`
  (`traits/impls.ts`), `through-entry-call-resolved`
  (`traits/calls.ts`), and `through-entry-args-for-the-impl`
  (`items.ts`).

## Amended by #803

`through_entry` is gone: a copied default's body is in its impl's terms
(ADR 0495, amended), so it calls the impl's method as the impl's own
methods do, `clicksTouch_me(clicks)`, its `&mut self` taken out of generic
code's box once, at its top.

## Amended (2026-11-06, #831): a box an entry takes apart isn't copied back

A call that gives an object a box for a trait's `&mut` copied the box
back once it returned, `$assign(a, self.value)`, as every box is (ADR
0099). Where the impl its `Self` selects has an entry that takes the box
apart, `EntryForms::unboxed`, the entry reads its `value` once and gives
its method, the impl's own or a default copied into it, the object,
which it changes in place: `*self = other` is `$assign(object, other)`,
`mem::take(self)` `$take(object, ..)`, and a generic callee it gives
`self` is given a box of its own, copied back into the object. Nothing
writes the caller's box whole, so its `value` is the same object, and the
copy back copied it onto itself. It's left out: decided by the entry's
owner, `traits/impls.rs`'s `entry_takes_box_apart`, of the impl
`codegen_select_candidate` selects and the forms its entry is made of,
never by the JS. A blanket impl's generic `Self` takes the box, writes its
`value` and is copied back, as are a `Self` no impl is selected for here,
an impl's own method of a type parameter, another crate's impl, whose
entry is its own, and a default the dictionary doesn't copy in: a
library's, compiled once over a generic `Self` (ADR 0100), whose
`*self = next` the gate's `crates.test.ts` caught, and std's.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, the same object
is assigned onto itself. Held by corpus `mut_method_pointer_replaced`
(defaults copied into an object's impl that replace, take and give
`self` to a generic `mem::replace`; a blanket impl's method and default
that replace `*self`, copied back), the eight snapshots it changes in
both builds, each against native Rust, and the mutations
`object-box-never-copied-back`, `entry-box-copied-back`
(`lower/mir/assign.ts`), `entry-takes-any-box-apart` and
`library-default-taken-apart` (`lower/traits/impls.ts`).


## Amended (2026-11-08, #968): a `&mut dyn` of an object is an in-place cell where an entry writes its box

A `&mut` to an object made a `&mut dyn Trait` was its pair, `{ value: c,
impl }`, and a `&mut self` method is given the pair as its box (ADR
0099). An impl's entry that takes the box apart (`entry_takes_box_apart`,
#831) gives its method the object, which it changes in place. A blanket
impl's, `impl<T: Default> Reset for T`, doesn't: its generic `Self` writes
the box whole, `*self = T::default()` is `self.value = ..`, and so does a
write through the `&mut T` that `impl<T> GetMut<T> for T`'s `get_mut`
gives back, the pair itself. Nothing copied the pair's `value` back, so
`c` stayed as it was: native `C { n: 0 }`, JS `C { n: 5 }`. Found by the
metamorphic net's `dyn-ref` rewrite (#939), 12 of its 16 rewrites.

Where a `&mut self` method of the `dyn`'s trait, or of a supertrait, has
an entry of the crate's that gives its method the box as it is (`entry_gives_box`,
the entry owner's answer beside `entry_takes_box_apart`), the pair is an in-place cell of
the object, as `$cellOf` is (above, #528): `{ impl, get value() { return
c; }, set value(value) { $assign(c, value); } }`. Its storage is
`Storage::Assigned`, which a pair carries as a handle does (`ExprKind::Pair`'s
third part): preparation writes its setter, and the linker asks for the
helper it writes with (`Storage::helper`). Such a handle never sets its
variable (`Storage::rebinds`), so `c` stays a `const`. A pair whose impls
all take the box apart, a concrete impl's, is `{ value: c, impl }` as it
was. A copy back where the borrow ends was the alternative: a `&mut dyn`
held by a `let` has no one end, and what `get_mut` gave is written after
the call returns. The user chose the in-place cell (2026-11-08).

A pair reads its place whenever its `value` is, so its place is held as
it was where it's made (`place_where_made`): a reborrow's, `&mut *x`, of
the `x` then, `const x$1 = x`, and an index a variable set again is,
`&mut lanes[index]` before `index += 1`, of the index then; tidy's
`alias_variables` keeps a temporary a handle's or a pair's place reads
where what it's of is set anywhere (`handles_read`).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change;
Rust's preserved. Held by corpus `dyn_of_a_generic_impl_written_whole`
(native-compared, both builds), `test/lowering-dyn-pairs.test.ts` (both
forms), the metamorphic net's 12 `dyn-ref` rewrites taken off its list,
and the mutations `generic-impl-pair-a-box`,
`generic-impl-pair-of-supertraits-only`,
`concrete-impl-pair-written-in-place`, `reborrowed-pair-place-unfixed`
(`lower/traits/dyns.ts`), `pair-place-holder-unnamed`
(`lower/mir/temporaries.ts`), `alias-read-by-a-pair-dropped`
(`lower/mir/tidy/bindings.ts`), `assigned-storage-rebinds`,
`assigned-storage-a-binding` (`js/storage.ts`) and
`storage-helper-unasked` (`link.ts`). Still open: a pair of a place whose
storage is a flattened state's keys or a nullable field's `null` (#977),
an associated type's `Iterator` dictionary (#972) and a returned `&mut`
copied in and out (#973).

## Amended (2026-11-08, #973, #975): what may keep a borrow past the call

A `&mut` given to a call whose result or another parameter may keep it is
the place itself, a handle or an in-place cell (#528 above), as
`borrow_outlives_call` answers. It read the lifetimes of the signature's
types alone, so three ways a borrow is kept were boxed and copied back
once the call returned, and a write through what was kept was lost:

- a lifetime only another parameter's bound names, `kept: &mut impl
  GetMut<Vec<&'a mut u32>>` with `x: &'a mut u32` (#973): the bounds'
  trait arguments and associated types are read too, not what outlives
  what;
- a type parameter given the borrow, which the result or another
  parameter names, `fn same<T>(t: T) -> T` given `&mut n` and
  `into_iter(self)`'s `Self::IntoIter` given `&mut C`: `*same(&mut n) +=
  10` was lost on `main` before this, native 11, JS 1;
- a `&'static mut` result where what's borrowed is a type parameter,
  `&'a mut impl GetMut<String>`, which may be what a transmute gives back
  (#975's first cause).

The user chose the one rule over keeping a box where the call's result
goes unread (2026-11-08): five snapshots that discard what the call gives
back give the place now, as any call that may keep it does.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change;
Rust's preserved. Held by corpus `mut_kept_by_what_a_call_gives_back`
(native-compared, both builds), the metamorphic net's rewrites taken off
its list (`dyn-ref` and `impl-ref` `into_iter_of_ref_param`, `impl-ref`
`mut_ref_escapes` and `transparent_transmute`), and the mutations
`bound-lifetime-unread`, `type-parameter-given-back-unseen` and
`static-result-of-a-type-parameter-unseen` (`lower/mut_refs.ts`).

## Amended (2026-11-08, #977): a pair on a handle stores as the handle does

A `&mut` to a place that stores what's written its own way, a
`#[rust_js::nullable]` field's `None` as `null` (ADR 0772) or a class's
flattened state's keys on its object (ADR 1044), is a handle that carries
its `Storage`, held in a `const` (`through_handle`). Made a `&mut dyn`, its
pair was on the handle's `value`, and tidy's `read_handles` read that as
the handle's place, so the pair's setter assigned the place as it's given:
`fiber.owner = value` left `undefined` where Rust's `None` is `null`, and
`task = value` replaced the variable holding the class's object instead of
setting its keys. Now a pair on a handle's `value` is a pair on its place
that stores as the handle does (`Expr::stored_pair`, #968's), its setter
`fiber.owner = value ?? null` or each key set.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change;
Rust's preserved. Held by corpus `stored_place_made_a_mut_dyn`
(native-compared, both builds), the metamorphic net's `dyn-ref`
`flattened_state_got_mut` and `nullable_written_through_borrows` taken off
its list, and the mutation `pair-of-a-handle-stores-as-given`
(`lower/mir/tidy/values.ts`).

## Amended (2026-11-08, #985): a pair's place is named once

#968's in-place pair took a value that reads the same for its place, and
an object made where it's borrowed, `&mut Count { n: 3 }`, reads the same
by that rule: each read of the pair's `value` made another `{ n: 3 }`, so
`while let Some(x) = s.next_item()` never ended (a regression of
f54145bb3, found by the metamorphic net run whole). MIR's lowering names
it once, `const count = { n: 3 }`, and tidy's `inlined` moved that
temporary into the pair's place, read once where it's written, but read
again whenever the pair is: it moves no temporary but a variable into a
handle's or a pair's place now (`handles_read`). Held by corpus
`temporary_made_a_mut_dyn_stepped` and the mutation
`temporary-moved-into-a-pair` (`lower/mir/tidy/moves.ts`).

## Amendment: a function given that takes the box itself is given it (#975)

A function of the crate's given for an `F: FnMut(&mut T)` of a `T` that's
an object here was always given generic code's box taken apart, `(x0) =>
f(x0.value)`. One whose own parameter there is a `&mut` of its own type
parameter's, `bump(c: &mut impl GetMut<Counter>)`, reads generic code's box
itself (ADR 0099), and was given the object: a `TypeError`.
`unboxed_by_function` asks the function given, by its type, and leaves the
box as it is where it takes one (`takes_box_itself`). Corpus case
`generic_fn_value_is_one_function`.

## Amended (2026-11-08, #988): every kept handle holds its place as it was made

#968 held a `&mut dyn` pair's place as it was where it's made
(`place_where_made`). A `&mut` to a number kept past where it's made, a
handle, reads its place whenever its `value` is read too, and two of a
place's parts may be set again while the borrow lasts, as borrowck lets
them: an index a variable holds, `&mut c[k]` before `k = 2` (native `[11,
2, 3]`, JS `[1, 2, 13]`), and the reference a field holds that the place
is reached through, `&mut (*h.node).count` before `h.node = &mut other`.
The one rule is where MIR makes the borrow, `Rvalue::Ref` of a `&mut` to
what's a cell: an index MIR copied from a variable set again, or one set
again itself, and a field's reference that the body assigns, name the
place's parts as they are there, `const k$1 = k`, `const node = h.node`;
a place of parts nothing sets again is as it was, no `const`. Found by the
metamorphic net's `boxed` rewrite (kept_element_ref, mut_ref_local).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change;
Rust's preserved. Held by corpus `kept_handle_of_a_place_set_again`
(native-compared, both builds) and the mutations
`kept-handle-index-read-later`, `kept-handle-field-reference-read-later`
(`lower/mir/rvalues.ts`) and `kept-place-object-unnamed`
(`lower/mir/temporaries.ts`).

## Amended (2026-11-08, #992): a `&mut` given for a parameter read as another is as that one takes it

A `&mut` given for a type parameter is generic code's box where its caller
has an object (above): `P` of `twice(f: impl Is<F>)` is bound by nothing
that calls or steps it. But its value is read as `F`, `let f: F =
f.is()`, a function bound by `FnMut()`, which takes a `&mut` closure as
the closure itself, and so with `I: Iterator`, which takes a `&mut` range
or iterator as itself: the box was called, `f is not a function`, and
stepped, `[]` where native gives `[1, 2]`. Now a `&mut` to a function or
an iterator given for a type parameter whose bound names one called or
stepped (`F`, `I: Iterator` or `I: IntoIterator`) is given as that one
takes it, the value itself (`given_as_the_value`, which `boxed_in_call`
and a call's arguments both ask); a collection given for an
`IntoIterator` is still its cell, as its dictionary's `into_iter` takes
it. Found by the metamorphic net's `impl-owned` rewrite.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change;
Rust's preserved. Held by corpus `mut_given_for_a_parameter_read_as_another`
(native-compared, both builds), the net's `impl-owned` impl_fn_mut_lent,
iterables_given_to_generics, nth_passes_a_slice_or_each and
references_given_to_generics taken off its list, and the mutations
`parameter-read-as-a-function-boxed` and
`collection-for-an-into-iterator-unboxed` (`lower/mut_refs.ts`).

## Amendment: a kept `&mut` of a type of one value is a box of it (2026-11-09, #1061)

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. A `&mut
()` given where the call may keep it, rustc's
`async-await/issues/issue-55809`'s `bad(&mut v).await` with `v: ()`, whose
future names `T`, was refused since this decision's amendment for #973 as
"a `&mut` of a number given back": its place isn't fixed, as a `()`'s value
is `undefined`, so there was no handle to give. No write changes a value of
a type of one value, `()`'s or a unit struct's, so nothing need be copied
back: it's given as a box of it, `{}`, kept or not, as a `&mut` of one is
where the call can't keep it. Held by the corpus case
`mut_of_one_value_kept` and the mutation `one-value-kept-mut-refused`; the
rustc test runs as native.

## Amended (2026-11-10, #1170): an object's box at a call is its in-place cell

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. A call
gave an object's `&mut` where generic code has its box as a box of its own,
`const t = { value: p }`, copied back once the call returned, `$assign(p,
t.value)`, in a `try` whose `catch` copied it back on a panic too; where the
call could keep the borrow, as an in-place cell, `$cellOf(p)` (#528), and an
entry that took the box apart wasn't copied back (#831). Every other
boundary, a `fn` pointer's, a function value's, a function given to generic
code (ADR 1115), gave the in-place cell. Two forms of one value, decided by
two rules: this decision's `boxed_in_call`, and `Form::Box`'s.

A call gives the in-place cell too: `reset($cellOf(p))`, whose write
changes `p` in place as it's made, as `*t = v` of the `&mut` does, so a
panic after it leaves `p` written, as Rust's does, with no copy back to
make or skip. What the callee moves out of the box is a copy apart already
(`mut_refs::moved_out`), as the box held the object itself. One rule says
where: `generic_facts.rs`' `Form::Box`, through shared references and a type
parameter given a `&mut` (`declares_box`), which `boxed_in_call`'s callers,
a call's argument, its result, a dictionary's entry's result and a
generic `&mut` nested in what's given, now ask. A box is copied back only of
a number's place (ADR 0074); #831's question of an entry that takes the box
apart, and the copy back's object flag, are gone with what asked them.

Held by the 54 corpus cases whose snapshots this changed (896 lines to 396,
each against native Rust), and the mutations `mir-object-unboxed`
(retargeted), `param-given-mut-raw` and `boxed-through-shared-unpeeled`
(moved to `lower/generic_facts.ts`), `fn-bound-mut-boxed` (new: a `&mut F`
of an `F: FnMut()` isn't boxed) and `cell-of-write-rebinds`
(`runtime/cell_of.ts`, new: the cell's write is in place), which replace
`mir-object-copied-back-rebound`, `outliving-object-handle`,
`object-box-never-copied-back` and `entry-box-copied-back`, whose code is
gone.

## Amendment: a `&mut` to a held `&mut` to an object is given in generic code's form (#1300)

A `W = &mut v` given by value is an in-place cell of the object,
`$cellOf(v)`, which `$mutWriter` (and #1156's `$mutReader`) read through.
But a `&mut` to a local holding that `&mut`, `let mut by = &mut v;
put(&mut by)` of `put<W: Write>(w: &mut W)`, boxed the local as it held
it, `{ value: by }`, the array: `$mutWriter`'s `w.value.value` read
nothing and the program threw, "Cannot read properties of undefined
(reading 'push')", where native Rust writes. One value, a `&mut` to an
object, had two forms where generic code took it.

- **The box holds generic code's form** wherever the callee's parameter is
  a `&mut` of its type parameter and the caller's is a `&mut` to a `&mut`
  to an object (`Form::Box` of the pointees, `form_differs`): `{ value:
  $cellOf(by) }`, and what's copied back is the `&mut` that holds,
  `form_back`'s `.value`, `by = w.value.value`, so generic code's `*w =
  other` puts `other`'s object in `by`.
- **One that's a box already**, a cell's place or a handle,
  `by.as_mut().unwrap()`'s, is read through: `$heldCellOf(cell)`, whose
  `value` is the in-place cell of what it holds, and whose write puts
  what's written's object in it.

`test/corpus/mut_of_a_held_mut_given_to_generic_code.rs`, a writer, a
reader, a struct's field, an array's item and one generic code replaces
whole, runs against native Rust in both builds and through its
metamorphic rewrites, whose `Option` rewrite found the cell's case.
What generic code gives back of such a `&mut`, `keep(&mut by)`'s, is
#1303.

## Amendment: what generic code gives back of a held `&mut` is the caller's (#1303)

`keep<W: Write>(w: &mut W) -> &mut W` given `&mut by`, `by: &mut Vec<u8>`,
gave back generic code's box, `$heldCellOf(..)`, whose `value` is the
object's in-place cell, where its caller reads a box whose `value` is the
object: `keep(&mut by).write_all(b"k")` threw `result.push is not a
function`, where native Rust writes. The one form is now a fact of the
catalogue's (`generic_facts.rs`), `Form::HeldBox`: a `&mut` of its type
parameter whose caller's is a `&mut` to a `&mut` to an object, the box of
a `Form::Box`. Given, it's `$heldCellOf(cell)`, as #1300 made it; given
back, `form_back`, it's `$heldObjectOf(box)` (src/runtime/cell_of.js),
whose `value` is the object and whose write puts the object's cell in
generic code's box, so `*keep(&mut by) = &mut other` makes `by` hold
`other`. `form_of` doesn't ask it: its askers, a function value's
parameters and a closure's, take a box or a handle, and none of them is
given a held box; `boxed_args` asks it of what's given, a place's box
and one held as a value, a closure's parameter, alike (which the
metamorphic `closure-fn` rewrite found, given as its caller held it), and
`generic_result_values` of what's given back. Case
([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior. `test/corpus/mut_of_a_held_mut_given_back_by_generic_code.rs`,
a `Vec<u8>` written through what's given back, given again and replaced
whole, a `String` and a struct, through a function value and by a
closure, runs
against native Rust in both builds and through its metamorphic
rewrites; mutations `held-box-given-back-as-it-is` and
`held-box-of-no-box` (`lower/generic_facts.ts`) and
`held-mut-value-given-as-it-is` (`lower/mir/assign/boxed_args.ts`).

## Amendment: a function value of generic code given a held `&mut` (#1322)

The #1303 amendment above said `form_of` doesn't ask `Form::HeldBox`
because its askers, a function value's parameters and a closure's, take a
box or a handle and are never given a held box. That was false. A `fn`
pointer of `stamp::<&mut Vec<u8>>`, given to a function of the crate's
that calls it, gave the instance its callers' box, whose `value` is the
object. The instance read that box as a box of the object's cell and
threw `Cannot read properties of undefined (reading 'push')`. Case
([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior. `form_of` asks all three forms now, so a function value adapts
a held `&mut` as a direct call does: `const stampRefMutVecFn = (arg0) =>
stampRefMutVec($heldCellOf(arg0));`, and what it gives back through
`$heldObjectOf`. A function value's form is its taker's
([1114](1114-a-reference-a-pointer-may-leave-with-is-its-places-pointer.md),
amended, #1324). Generic code's `F` of a held `&mut` is still refused, as
any `&mut` to an object inside generic code's parameters is. Held by
`generic_fn_value_takes_its_callers_forms.rs` and the mutation
`held-box-form-unasked`.

## Amendment: `entry-takes-any-box-apart` is held by a `&mut dyn` (2026-11-11)

Since 1aefc906e a function pointer of a blanket impl's method gives the
box in generic code's one form, so `entry_gives_box` is asked only of a
`&mut dyn` made of an object (`dyns.rs`' `pair_written_whole`, #968), and
`mut_method_pointer_replaced` no longer told `entry-takes-any-box-apart`
apart: it survived on main. It's held by corpus case
`dyn_of_a_generic_impl_written_whole`, whose `&mut dyn` of a blanket
impl's `*self = ..` leaves the caller's object as it was with the mutation
(`C { n: 5 }` where Rust's is `C { n: 0 }`), as crate pair
`library_singles_across_crates` does too.

## Amendment: every dictionary of a `&mut` reads generic code's box (#1414)

A dictionary of a `&mut X`, given where a type parameter is, is read with
generic code's box of `X`: `$cellOf(object)`, or its caller's cell of a
number or text. A crate's impl for `&mut T` read it so, as did `AsMut`,
`$mutWriter` and `$mutReader`. Three routes didn't, each a wrong answer
(P0, #1414). `run(&mut parser)` of a `P: BorrowMut<Parser>` wrote the box,
leaving the parser as it was, and `show(&mut point)` of a `D: Display`
showed `undefined`:

- **std's `Borrow` and `BorrowMut` of a `&mut`** give what the box holds,
  `(value) => value.value`; a `&mut u32`'s too, which was refused.
- **std's impls that pass a trait on from `&mut T` to `T`**, `impl<T:
  Display> Display for &mut T`, `Debug`, `Hash`, `AsRef` and the rest, give
  `T`'s dictionary through the box. `$throughBox(pointDisplay, { fmt: [1]
  })` gives each entry what each box it takes holds: `1` a `Self` or a
  reference to one, `2` a slice of them. The supertraits' entries are the
  `&mut`'s too. An entry that gives a `Self` back, or a trait of another
  `&mut`, is refused. An iterator trait's `&mut I` is given as it is (ADR
  0061), and so is a `&mut dyn`'s pair, never boxed again.
- **A `dyn` made of a `&mut` to an object**, `&r as &dyn Total`, holds the
  box, `{ value: $cellOf(r), impl }`, as a type parameter given it does.
  Its impl for `&mut T` read the object as the box and threw.

A std struct that holds a `&mut` to an object as the object, a
`Cursor<&mut Vec<u8>>` made where its type is known, is given the object's
`AsRef`, which std's of the `&mut` passes it on to. Case C of ADR 0262:
results change, so Rust's behaviour is kept. Corpus case
mut_ref_given_for_a_bound covers each, against native Rust in both builds.

## Amendment: a struct's field of a `&mut T` holds generic code's box everywhere (#1405)

A field of the crate's own struct or enum whose declared type is generic
code's box, `at: &'a mut T` or `parser: P` given a `&mut`, held the box
where generic code made it and the object where its caller did, and it
was refused wherever one crossed into the other: `fn hold<T>(at: &mut T)
-> Slot<'_, T>`, a guard, regex-syntax's `ParserI<&mut Parser>`, 945 of
the Crater's tests. The field has one form now, decided by its declared
type, the user's decision: **generic code's box wherever the type is**.
Code that knows `T` adapts it at the field's crossing, `field_crossing`,
the boundary API of a call's `Form::Box`:

- it's made the object's in-place cell, `Slot { at: $cellOf(point) }`, and
  written whole so too, `slot.at = $cellOf(other)`;
- it's read through `.value`, `slot.at.value.x += 10`, and read whole as
  what the box holds, `let Slot { at } = slot`, `&slot.at`;
- a `ref mut` that names the field where it's used, `let r = &mut
  slot.at`, reads and writes `*r` at that crossing too.

A `&mut` to the field given to a function, `repoint(&mut slot.at, q)`,
would be to the box: it's refused, naming the field. A number's field is
a cell either way and doesn't change. What a field holds within a part,
`Option<&mut T>`, a `Vec<&mut T>`, is still refused across generic code.
The cost is one `.value` deeper where `T` is known. Case C of ADR 0262:
results change, so Rust's behaviour is kept. Corpus case
mut_ref_fields_of_a_type_parameter covers a slot, a guard, a holder,
regex-syntax's parser holder, an enum, and impls bound by `Iterator` and
`FnMut`, against native Rust in both builds.

## Amendment: a `&mut` to the code's own type parameter, through std's impls (found by #1293)

#1414 gave std's impls that pass a trait on from `&mut T` to `T`
(`AsRef`, `Display`, `Debug` and the rest) `T`'s dictionary through the
box, `$throughBox`, but only for a `&mut` to a type the code knows. It
asked whether that pointee is a function or an iterator, and a bare type
parameter answers "may be an iterator". So generic code's own `held:
&mut T` gave `T`'s dictionary the box itself. `held.as_ref().len()` was 0
where Rust's is the text's length; the box's `value` was never read.

A `&mut` to a type the code doesn't know is the box this code holds it
as (`mut_ref_form`), unless that's a function or an iterator given as it
is (ADR 0061), as `in_generic_box` now says. Case C of ADR 0262: results
change, so Rust's behaviour is kept. Corpus case
mut_ref_to_its_own_type_parameter_through_std_impls covers `as_ref()`
through a `&mut T` the code was given and one a `Result`'s `as_mut()`
lends, written back through, `{}` and `{:?}` of a `&mut T`, and an
iterator's `&mut I` given as it is, against native Rust in both builds.
