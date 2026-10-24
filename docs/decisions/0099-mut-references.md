# 0099. A `&mut` held in a variable names its place; one kept elsewhere is a handle

Status: Accepted in part: a `&mut` in a variable, the index loop, a
generic `&mut T`, a `&mut` to a closure, a `&mut dyn Trait` of the
crate's, and handles of a `&mut` to a value JS can't change in place, and
to a temporary. A generic `&mut T` to
an object inside what a generic function takes or gives, and a handle to
an object replaced whole, are to come. Extends
[0049](0049-traits-and-generics.md), [0025](0025-vec-loops-refcell-mut.md), [0033](0033-enums-with-fields.md) and [0074](0074-mut-boxes.md).

Case: A, C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `&mut` to a JS object is the object (ADR 0025), and a `&mut` to anything
else, a number say, is a box `{ value }` when it's a parameter, copied back
after the call (ADR 0074). A `ref mut` binding of a field names the field
(ADR 0033). Any other `&mut` to a value that isn't an object is an error:
one held in a variable, kept in a struct or a `Vec`, or returned.

It's the value rustc's tests stop at most: 70 of them. By the type they
stop at:

| `&mut` to | Tests | Like |
|---|---|---|
| a number, `()`, a `&x`, a `Box` of one, a `String` or an `Option` | 37 | `let y = &mut x; *y = 5;`, `for i in &mut ints { *i += 22 }` |
| a type parameter, or `Self` in a trait | 13 | `fn to_refs<T>(list: &mut List<T>) -> Vec<&mut T>` |
| a `dyn Trait` | 12 | `let w: &mut dyn Write = &mut out;` |
| a closure or a future | 6 | `fn call<F: FnMut()>(f: &mut F) { f() }` |
| a struct or a union of the test's own | 3 | |

In most, the `&mut` never leaves the function that made it: a variable
holds it for a few lines, or a loop gives one for each item. The rest keep
it, in a `Vec` or a struct, or return it.

## Decision

(Amended: a place reached through a `&mut` or a raw pointer, `e.n` of an
`e: &mut N`, isn't a stable one, read as it is later: what's done
meanwhile writes it through the same reference, as `<p>{e.n}{{ e.n = 5; 1
}}</p>` did, reading 5. It's read first.)

**A `&mut` a variable holds names its place,** as a `ref mut` binding does
(ADR 0033). `*r` is the place, read or written:

```rust
let mut x = 3;
let y = &mut x;
*y = 5;
println!("{}", *y);
```

```js
let x = 3;
x = 5;
console.log(`${x}`);
```

- **It's exact.** While `y` lives, Rust lets nothing else use `x`, so a
  write through `y` is a write to `x` that nothing can tell apart.
- The place is fixed where it's borrowed: an index is evaluated then,
  once, `let r = &mut v[i]` keeping `i`'s value if `i` could change, and a
  place through an object keeps the object, not the variable that held it.
- It holds for a variable bound once, by `let`, to `&mut` of a place or a
  reborrow of one, and used as `*r`, a method's receiver, or passed on, as
  `&mut *r` is. Passed on, it's the place given to a call, boxed as ADR
  0074 boxes one.
- `*r = v` replaces the whole value, an object's too, which ADR 0025
  refused: it's `x = v`.

**`for x in &mut v`, of values that aren't objects, is an index loop,** and
`*x` names `v[i]`:

```js
for (let i = 0; i < ints.length; i++) {
  ints[i] += 22;
}
```

For an array, a `Vec` or a slice, and `iter_mut()` of one. Of objects it
stays `for (const todo of s.todos)` (ADR 0025).

**A `&mut` kept anywhere else is a handle:** in a struct, a `Vec` or an
`Option`, in a variable that's assigned again, chosen by a branch, or
returned. It reads and writes its place:

```js
{ get value() { return x; }, set value(value) { x = value; } }
```

- It has a box's `value` (ADR 0074), so a function taking a `&mut` reads it
  the same way whichever it's given.
- Its place is fixed when it's made, as a variable's is: `&mut list.value`
  keeps the `list` object of that moment, `const o = list;`, so moving
  `list` on doesn't move the handle.
- A call that can keep its parameter's borrow, one whose return type, or
  another parameter's, names the borrow's own lifetime (not what it
  refers to's: `take_word<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]>`
  gives back a slice, keeping nothing of the borrow), is given a handle,
  not a box: the box is copied back when the call returns, and the borrow
  outlives it. `keep<'a>(v: &mut Vec<&'a mut u32>, x: &'a mut u32)` keeps
  `x` in `v`, and `hold(&mut self, x: &'a mut u32)` of a `Holder<'a>` in
  `self` (`test/corpus/mut_ref_escapes.rs`); a box of either gave a
  wrong answer, the writes through it after the call lost.
- A reborrow of a reference reassigned after, `&mut *x` before `x = z`,
  named where it's used, `x.value`, is of what `x` was, held where `x` is
  reassigned, `const x$1 = x`: `add(&mut *x, { x = &mut q; 10 })` added to
  `q`, a wrong answer (`test/corpus/mut_ref_reborrow_reassigned.rs`).
- Of an object, a handle's `value` is the object, and changes through it
  are the object's; only a `&mut` to an object that's kept, and replaced
  whole through, needs one.
- **A cell compared, matched or shown is what it points at:** `p == q`,
  `p < q`, `{p}`, `{v:?}` of a `Vec<&mut i32>`, its `sort` and its
  `contains` are of the numbers. JS's `===` and `<` of two cells would
  compare the objects, and `${p}` show `[object Object]`.
- **A std call's own `&mut` is the item itself, not a cell:**
  `m.get_mut(&k)`'s, `iter_mut()`'s items'. std's JS gives the value,
  not a place. A pattern matching the call takes it apart, `Some(v) => *v
  + 1` reading the item; kept, passed on or given to a closure, it's an
  error, and so is `v` as a value, which would be a handle on the
  binding's copy. A `&mut` a std call only passes on, `refs.pop()` of a
  `Vec<&mut i32>` or `o.unwrap()`, is still the crate's cell: one its
  arguments or its type's parameters hold. (Amended: used as a value, a
  std call's `&mut` to a number or a string is a handle on the item, ADR
  0152.)

**A `&mut` to a temporary is to a `let` of its own:** `&mut 3`, `&mut
f()`, `let r = &mut 0;`. The temporary is `let tmp = 3;`, and the `&mut`
names it, as rule 1's does, or is a handle on it where it's kept. Given
to a function, it's a box, `{ value: 3 }`, not copied back: nothing else
can see it.

**A generic `&mut T` is a box or a handle whatever `T` is.** A generic
function is compiled once, and its `T` might be a number, so `*r` is
`r.value` in it. A caller whose `T` is an object gives it a box too, and
takes the value back after. Kept or returned, a generic `&mut T` is a cell
too, as a `&mut` to a number is, and a caller whose `T` is a number has
it as one. For a caller whose `T` is an object, whose own `&mut` to one is
the object, a `&mut T` returned is what's in the cell, `pick(..).value`.
One inside what a generic function takes or gives, a `Vec<&mut T>`, an
`Option<&mut T>`, a field of the crate's own type or a closure's
parameter, is an error for an object `T` for now: nothing converts it.

- **A trait's `&mut self` method, through its dictionary, takes a box** (ADR
  0049), since generic code calls it with its `&mut T`: an object's impl's
  entry takes what's in it, `bump: (self) => cBump_bump(self.value)`. A call
  that knows the impl calls its method as it is, given the object.
- **A trait's default, copied into an impl, takes its `&mut self` as a
  box too:** it's in the dictionary. Calling the impl's own methods, it
  gives an object's what's in the box.
- **A `&mut dyn Trait` gives its `&mut self` methods a box, its pair
  itself:** a trait object's methods are its impl's dictionary's, and its
  pair, `{ impl, value }` (ADR 0049), has a box's `value`. `d.bump()` is
  `d.impl.bump(d)`, and a `&self` method is given `d.value`. Of a `&mut`
  to a number, the pair reads and writes its place, as a handle does:
  `{ impl: i32Counter(), get value() { return n; }, set value(value) { n =
  value; } }`. Of an object, it's `{ value: c, impl }`, and a `&mut` to a
  `dyn` is the pair, as one to an object is the object. A `&mut dyn Sub`
  as a `&mut dyn Super` is a pair on the first's `value`.
- **Not a type parameter that's a JS function or a JS iterator already:**
  one bound by `Fn`, `FnMut` or `FnOnce`, whose `&mut` is the closure, or
  by `Iterator` (ADR 0061).

**A `&mut` to a closure is the closure,** a type parameter bound by `Fn`,
`FnMut` or `FnOnce` too: calling a JS function changes what it captured,
as calling it through the `&mut` does in Rust. `f()`, not `f.value()`.
Assigning a new closure through one is an error.

**Not here:** a `&mut dyn` of std's traits, `Write` or `Iterator`, raw
pointers, and a `&mut` in a `static`.

## Why

- **It's what a person writes.** `x = 5` for `*y = 5`, and an index loop to
  change an array's numbers. A handle, where one's needed, is the object a
  person would make to pass a variable around: React's `useRef` is one.
- **It's exact.** Each form is the place, for as long as Rust lets the
  `&mut` be used, and the borrow checker keeps anything else from using
  the place meanwhile.
- **It's small where it's common.** Most of these tests only hold a `&mut`
  for a few lines, which needs no new shape at all.

## Alternatives

- **A handle for every `&mut`:** one rule, but `y.value = 5` for `*y = 5`,
  where the place itself reads as the Rust does.
- **Boxing every variable a `&mut` is taken to** (`let x = { value: 3 }`):
  ADR 0025's alternative, with every read of `x` a `.value`.
- **A place as a pair,** `[object, "key"]`: small, but `r[0][r[1]] = 5`
  reads worst of all, and a variable isn't a key of anything.
- **A copy of each generic function for each `T`:** `*r` would be the
  place for an object and a box only for a number, but rust-js compiles a
  generic function once, given what depends on the type (ADR 0049).

## Consequences

- **The first two rules are in,** each with its corpus cases, compared
  with native Rust, and its mutations:
  - `mut_ref_local`: a number, a `String`, an `Option` and a field
    through a `&mut` in a variable, one passed on and one reborrowed; an
    index fixed where it's borrowed, `const at = $at(v, i);` and then
    `v[at]`. Reached through a reference that can be assigned again, one
    anywhere but an immutable variable (a `let mut`, a field, an element,
    behind another `&mut`), the place is in the object of that moment,
    kept: `const o = cur;` for `&mut cur[0]`, `const o = h.list;` for
    `&mut h.list[0]`, `const o = refs[0];` for `&mut refs[0][0]`. Rust
    freezes the rest of the path while it's borrowed, but not where a
    reference is.
  - `mut_ref_loop`: `&mut v` and `iter_mut()` of a `Vec`, an array and a
    slice, `continue` and `break`. The loop keeps the collection it starts
    with, `const items = cur;`, if what holds it is assigned again. Of
    `&mut v[a..b]`, it runs from `a` to where `$sliceEnd` says it ends,
    which panics as `$slice` does (`mut_ref_loop_bounds`).
- **A `ref mut` binding of a `let` variable writes it too,** as a `&mut`
  in a variable does, for a value that isn't an object: `if let Some(n) =
  p { *n += 1 }` is `o += 1`. Of an object, its variable may be a `&mut`
  itself, and assigning it wouldn't replace what it points to: that stays
  an error.
- **A trait's `&mut self` method on a number or a `String` is called as a
  function taking one is:** its impl's method takes a box (ADR 0074), so
  `n.bump()`, which resolves to it, boxes `n` and takes it back after
  (`trait_mut_self_value`).
- **A generic `&mut T` given as a parameter is a box** (`generic_mut_ref`):
  `twice(&mut c)` of an object boxes `c` and takes it back, `set(x, v)` is
  `x.value = v`, `t.bump()` of a generic `t` boxes it for the dictionary,
  and a default copied into two impls takes a box in each. Of rustc's 15
  tests stopping at a `&mut T` or a `&mut Self`, the two with a default
  `&mut self` pass; the rest return or keep one, which is a handle's.
- **A `&mut` to a number given where a `T` goes is a box too** (ADR 0074,
  `mut_ref_as_generic`), and one already in a box is the box: an impl for
  `&mut i32`, taking it by value, changes the number. Found by rustc's
  `issue-55809.rs`, which stopped when only a declared `&mut T` was boxed.
- **A `&mut` kept is a handle** (`mut_ref_handle`, `mut_ref_kept`): in a
  struct, written through as it's dropped, chosen by a branch, in a `let
  mut` assigned again, in an `Option` and a `Vec`, returned, and from a
  user `IndexMut`. It's a new kind of JS expression, `Handle(place)`,
  printed as the getter and setter above: its place is read when it's
  used, never before. A `&mut` to one of these values, as a value, is a
  cell, a box or a handle: `*r` of a variable holding one, a field keeping
  one, a branch or a call of the crate's giving one back, is `r.value`. A
  std call's isn't one: `v[i]`'s `index_mut` is the item itself. A call
  whose return type names a parameter's lifetime, `pick(&mut a, &mut b)`,
  is given handles, not boxes copied back before its result is used. A
  `&mut` in a variable (rule 1) kept is a handle on its place. `for r in
  refs` of a `Vec<&mut i32>` isn't an index loop: its items are cells. Of
  rustc's 45 tests stopping at such a `&mut`, 8 pass; one that then
  compiled and gave another answer, `issue-25515.rs`, had an `Rc<dyn Send>`
  of a value with a destructor that never ran, which is refused now, as
  `Box<dyn>` of one is.
- **A cell is what it points at wherever it's compared or shown**
  (`mut_ref_compare`): a variable's place, a box, a handle and a
  temporary's box, alone and in a `Vec`, an `Option` and a struct.
  Formatting, `==`, `Ord`, `sort`, `max` and `contains` look through
  each `&mut` that's a cell, and the fast paths, JS's `===`, `<` and
  `includes`, aren't taken with one. `&y` of a `let y = &mut x` is a
  handle on `x`, as `y` is, and so is `contains(&y)`, which Rust
  reborrows as `&*&y`. `match r { v => *v += 1 }` keeps the cell, and a
  `&v` pattern looks through it. `dedup` and `binary_search` of cells are
  errors for now. Found in review of the handles: each compared or showed
  the cell itself, and gave another answer than Rust's, with no error.
- **A std call's `&mut` is its item** (`mut_ref_std_items`, and the
  diagnostics test): found by rustc's `nll/process_or_insert_default.rs`,
  which passed before the handles and gave another answer after, its
  `Some(value)` of `map.get_mut(&key)` read as `value.value`. The handles
  had also let `get_mut` kept or passed on, `iter_mut().next()`,
  `for_each`, `collect()` and `values_mut()` compile, each reading the
  numbers as cells; these are errors again. An operator's call, `v[i]`'s
  `*index_mut(&mut v, i)`, which THIR marks as not the source's, gives
  its item where it's read or written, as before.
- **A `&mut` to a temporary is a `let` of its own** (`mut_ref_temporary`):
  `&mut 3` in a variable, a pattern's subject, kept, and given to a
  function, which gets a box it doesn't give back. `f::<isize>(&mut
  None)` of rustc's `regions-lifetime-static-items-enclosing-scopes.rs`
  then compiled, and showed `assert_eq!(*o, None)` of a box comparing the
  box: a tuple's parts, `assert_eq!`'s and `match (a, b)`'s, looked
  through `&*o` to `o`. They look through the `&` only now
  (`mut_ref_box_parts`).
- **A `&mut` to a closure is the closure** (`closure_mut_ref`): of a type
  parameter bound by `FnMut`, an `impl FnMut`, a `dyn FnMut` and a function,
  `&mut square`. `f()` calls it, and `call(f)` passes it on. A `dyn FnMut`
  isn't a box (ADR 0072): it's a function, not a value JS can't change.
  Assigning a new closure through one is an error. The 8 of rustc's tests
  stopping at one pass.
- **A `&mut` to an object in a variable is still the object** (ADR 0025),
  and `*r = v` of one still an error: replacing an object whole through a
  `&mut` comes with handles.
- **A generic `&mut T` kept or returned is a cell** (`generic_mut_ref_kept`,
  and the diagnostics test): `result.push(&mut list.value)` of rustc's
  `nll/mutating_references.rs`, `pick(c, a, b)`, `&mut v[0]`, and a
  `DerefMut` of the crate's. `pick(..).value` for a caller whose `T` is an
  object; a `&mut T` to one inside a parameter or the result, found by
  probing, is refused: a generic struct's field built by the caller, a
  `Vec` or an `Option` of them, and a closure's, which generic code gave
  a box of the object, and whose change went to the box, before this.
  `o.as_ref()` and `o.as_mut()` of an `Option` are the `Option`, through
  its cell, and `push_str` writes through a call's cell as `+=` does.
  Found by rustc's tests once they compiled: a `DerefMut` resolved to the
  crate's impl wasn't taken for the crate's, and a default copied into an
  object's impl made `&mut self` a handle on its local, which is refused
  again. Of the 12 tests stopping at a `&mut T`, 8 pass.
- **Anything else stays an error:** a generic `&mut T` to an object inside
  what a generic function takes or gives, and a `&mut dyn` of std's traits.
  (Amended: an object replaced whole through a `&mut` is ADR 0147's, in
  place, without a handle.)
- Most of the 37 tests of `&mut` to a value that isn't an object need the
  first two rules only, and those come first; handles, generic `&mut T`
  and closures follow, each with its corpus cases and mutations.
- **A `&mut dyn Trait` of the crate's is its pair** (`dyn_mut`): of an
  object and of a number, a parameter, a variable, a temporary, one made
  `&dyn`, and upcast to a supertrait: probing found an upcast writing a
  copy of the `value`. A `Box<dyn Trait>` owns its pair's `value`, which its
  `&mut self` methods write. The traits test's mutable `dyn` receiver,
  once an error, compiles. Of rustc's 10 tests stopping at a `&mut dyn`,
  4 pass; 4 are of std's `Write` and `Iterator`.
- Each test that gets further may stop at something else, as rustc's tests
  do; the known failures say where.

## Since

- **Preparation writes a handle**, `{ get value() { .. }, set value(v) { ..
  } }`, its setter a `Prop::Setter`, its parameter `value`, or `newValue`
  where the place reads `value` (`handle_named_value`); the printer prints
  the object. (Amended: the printer wrote it, choosing that name.)


## Amendment: a trait's `&mut self` method as a value

A trait's method of a `&mut` to an object, `Tally::bump` as a `fn(&mut
Tally, i32) -> i32`, was an arrow that gave the method the object as it
is, `(arg0, arg1) => tallyCounter().bump(arg0, arg1)`, where the method
takes a box of it, `{ value }`: a default's `self.value` read `undefined`,
and crashed. Such a method as a value is now the arrow of its call as MIR's
lowers it, which boxes the object, and copies the box back in the arrow
once the call returns, as a call's are after it: a default that replaces
the whole of `*self` replaces the object's contents. One of a `&mut` to a
number, a cell already, a box, is the direct arrow, given as it is. An
arrow that only calls a function of the crate's by name, given its
parameters as they are, is that function, wherever it's made, as
`called_by_name` says: not a runtime helper's, whose arrow names what it's
given by its type. The `mut_method_pointer` corpus case calls a default's
and an impl's, of an object changed in place, replaced, and of a number,
with native Rust.

## Amendment: a `&mut` of one value, and a cell a place holds

- **A `&mut` to a type of one value**, a zero-sized type's (`()`, a unit
  struct, `[S; 1]` of one, `has_one_value`), given where its place isn't
  fixed, an item of what a call gave, `call_and_ref(..)[0].mutate()`, was
  refused, as a box of it couldn't be copied back. None needs to be: no
  write changes one. It's a box of the value, evaluated once, not copied
  back. This replaces the test of a `()`'s place, which it covers.
- **A reborrow of a cell a place holds**, `&mut **u` of a `ref mut u` into
  an `Option<&mut T>`, is that cell, as a variable's is: `return o.value`,
  not a new handle on `o.value.value`. Which places hold a cell, a
  variable's or one a reborrow found, is one list, `cells`, which
  `value_expr` asks.
- The `option_as_mut_generic` corpus case gives a unit struct's `&mut`;
  `ref_mut_of_mut_payload` returns a reborrowed cell.

## Amendment: a `&mut` a call gives inside what it gives

**A `&mut` generic code has as a cell, given back inside what a call
gives, `store.lm_get_mut(i).unwrap().1` of an `Option<(&K, &mut V)>`, is
that cell, kept or handed on as it is**: a part of the call's result,
through fields and constant indices, `$unwrap(..)[1]`, made there once,
as the call's own result is. litemap's `get_mut` hands one on so.

## Amendment: a `&mut` shown is what it points at

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the text is the same;
only the JS reads better.

**A `&mut` formatted, `println!("{r} {r:?}")` of `let r = &mut n`, shows
`n`**: `${n} ${n}`, where it was a handle on `n` held in a variable named
`value` and read through its getter. Two parts make it so:

- A format argument is read as it's shown where it's made, a handle the
  place it's on: through its references, and its pins, counted `Rc`s and
  guards (`shown_through`, the one place `{}`, `{:?}` and a format argument
  all ask; `format_value`'s own read through references went with it). A
  pointer's `{:p}` is its address, as it is.
- A shared borrow of a kept `&mut`, `&r`, which `format_args!` takes, is
  held as a handle on its place, read where it's used, as a shared borrow
  of a place is. Only a shared one: a `&mut` given on, `f(&mut *t)`, is the
  cell it is, not a new handle each time it's read.

`mut_ref_local`, `mut_ref_compare` and `pin_box` show a `&mut` and a
pinned `&mut` so; `function_given_for_a_mut_param` gives a reborrow twice.
Mutations `fmt-arg-through-refs`, `format-without-cells`,
`held-shared-ref-handle` and `held-mut-ref-handle`; `rc_counts` shows a
counted `Rc`'s `to_string()`, which `rc-debug-box` mutates.

## Amendment: a returned `impl FnMut` is its function too

`is_boxable` read a closure's type as it's written, and one returned as
`impl FnMut` is an opaque type, not the closure: its `&mut`, lent to
generic code or to a `&mut dyn FnMut`, was a box of it, `{ value: f }`,
which was called as the function, "f is not a function" (#105). The type
is revealed first, so a returned closure's `&mut` is the function, as a
closure made here is. ADR 0262's case: it threw, so Rust's. Held by
`test/corpus/impl_fn_mut_lent.rs` and the mutation
`boxable-opaque-unrevealed`.

## Amendment: a `&mut` of a field or an item reached another way

`inc(&mut es[0].n)` and `fn first(v: &mut Vec<(i32, i32)>) -> &mut i32 {
&mut v[0].0 }` were refused (#120): the place, `$index(es, 0).n`, isn't a
fixed one, so a box of it couldn't be copied back, nor a handle made.
The object holding the field or the item is the same however it's
reached, and Rust reaches the place where it's borrowed. Given, that
object is named there, `const item = $index(es, 0)`, and the place copied
back through it; kept or returned, it's `$handleOn(object, key)`, the
object reached once, where it's borrowed, the same while the borrow
lasts, so an index past the end panics there, as Rust's does. Held by
`test/corpus/mut_field_given.rs`.

## Amendment: a handle a call made is a place reached another way

Once a field or an item reached another way is named where it's borrowed,
a handle's `value` a call made, `*first_mut().unwrap()`'s, is one of
them: its object, the handle, named there, so the place is fixed. The
branch that gave such a handle on as it was, `h.value`'s `h`, was then
never reached: no place is still a member or an item that isn't fixed
once that's done. It's gone, and a `&mut` of a place that still isn't
fixed, one neither named nor a member or an item, is refused as before.
Its mutation, `mir-handle-boxed-by-value`, went with it. `bump(
row.first_mut().unwrap())` is still `bump($unwrap($mutAt(row, 0)))`:
what MIR moves, the handle, is given as it is before any place is asked.

The `&mut` nothing reads, `let _ = &mut p.y` through `deref_mut`, would
now be made too, `$handleOn(derefMut(p).value, "y")`, where only
`derefMut(p)` is needed; that it isn't is how it reads, so its mutation,
`mir-unread-ref-handled`, is checked by `unread_mut_through_deref`'s JS.
