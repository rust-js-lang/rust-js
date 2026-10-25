# 0897. An element a guard reaches is its place, and `vec![..]` borrows nothing

Status: Accepted. Extends [0099](0099-mut-references.md) (a `&mut` to a
place is the place) to a parameter's index and to a `RefCell`'s guard a
temporary holds, and [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)
(what a call may borrow) to an intrinsic; amends
[0863](0863-a-refcell-of-an-object-is-the-object.md)'s guards.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. Each place is the one Rust's borrow is of: borrowck keeps what
it's through unchanged while it's borrowed, and where a reference it's
through is reassigned meanwhile, what it held is kept. A borrow no other
can meet is left unchecked only where none can; an index stays checked,
with Rust's message.

## Context

React's ReactFiberLane keeps its lanes' tables in `FiberRoot`'s
`RefCell<Vec<..>>` fields, the objects they hold (ADR 0863), and writes
them as upstream does:

```js
root.entanglements[index] |= lane;
..
const hiddenUpdates = root.hiddenUpdates;
const hiddenUpdatesForLane = hiddenUpdates[index];
if (hiddenUpdatesForLane === null) {
  hiddenUpdates[index] = [update];
} else {
  hiddenUpdatesForLane.push(update);
}
```

Its port's natural Rust, `root.entanglements.borrow_mut()[index] |= lane`
and `match &mut hidden_updates[index] { None => hidden_updates[index] =
Some(vec![update]), Some(list) => list.push(update) }`, printed a getter
and a setter of each element:

```js
$index(root.entanglements, index);
const value = {
  get value() {
    return root.entanglements[index];
  },
  set value(value) {
    root.entanglements[index] = value;
  },
};
value.value = value.value | lane;
```

and, of the `match`, a counted borrow, `$borrowMut(root.hiddenUpdates,
true)` in a `try`/`finally`, though nothing else borrows it. Three things
made them:

- **A `&mut` to an element is the place only where its index reads the
  same while it's borrowed** (`same_place`): a constant, or a variable
  written once. A parameter is written by its caller, never in the body,
  so it counted none, and its element was a handle on the place.
- **The guard a temporary holds, `borrow_mut()`'s, was a variable**: MIR
  borrows it, `&mut guard`, to give `deref_mut`, which counted as a
  write, so it was written twice, and named, `const e = root.e`, where a
  `RefCell` that's the object it holds has the guard be the object, read
  where it's used.
- **`vec![x]` calls an intrinsic**, `write_box_via_move`, the
  toolchain's `#[rustc_box]` that moves `x` into its box. An intrinsic's
  instance isn't an item, so what it may run was anything: a borrow held
  across `vec![..]` was counted, and so was every borrow of its type.

## Decision

- **An index variable given one value reads the same**: written once, or
  a parameter never written in the body (`given_once`). Its element, a
  `&mut` made once, is the place, `v[index]`, as a constant's is.
- **A `&mut` of a `RefCell`'s guard given at once to `deref_mut` alone
  writes what it guards, never the guard** (`locals.rs`'s
  `derefs_guard`): it isn't a write of the guard, as a `&mut` to a pin
  isn't (ADR 0329). So a guard a temporary holds is written once, and,
  of a `RefCell` that's the object it holds, unchecked, is that object
  where it's read, as any temporary's value is, and the `&mut` its
  `deref_mut` gives the place: `root.entanglements[$at(root.entanglements,
  index)] = root.entanglements[index] | lane`. A guard a `let` holds keeps
  its name, `const hiddenUpdates = root.hiddenUpdates`, as upstream's
  does.
- **A place a temporary is read in, held through a reference that's
  reassigned while it's borrowed, is made of what the reference held**
  (`hold_refs_through`, as for a `&mut` named where it's used): `s.v[{ s =
  o; 0 }] |= 4` writes the first `s`'s `v`. A temporary `&mut`'s place
  was read in place after the reassignment, `o.v[0]`, a wrong answer this
  change found and fixes.
- **A value written to a place of variables each given one value is
  written there, made where the assignment is**, `lanes[index] = (lanes[index]
  ?? 0) | lane`, no `const value` before it (`settled_place`): JS reaches
  the place first, but nothing the value runs can make it another, as no
  closure it calls assigns one of its variables, which would be a second
  write. A place a check reaches, `v[$at(v, i)]`, or of a variable written
  again, keeps the value made first, as Rust makes it (ADRs 0607, 0879).
- **An intrinsic runs only what it's given**: what its bounds say, as
  std's code does (`Called::Std`); of a function pointer or a `dyn`,
  `catch_unwind`'s, which may be any, anything. `write_box_via_move`
  runs nothing.

```rust
use std::cell::RefCell;

pub struct Root {
    pub entanglements: RefCell<Vec<u32>>,
}

pub fn entangle(root: &Root, index: usize, lane: u32) {
    root.entanglements.borrow_mut()[index] |= lane;
}
```

```js
export function entangle(root, index, lane) {
  root.entanglements[$at(root.entanglements, index)] = (root.entanglements[index] | lane) >>> 0;
}
```

How an assignment of an operator is printed, `x = x | y` or `x |= y`, is
the printer's, apart from the place.

## Why

- **Upstream's JS**, the element written where it is, no object of a
  getter and a setter around it.
- **The same place**: the index is the parameter's value throughout, and
  borrowck keeps the vector it's in unchanged while the element is
  borrowed; the guard is the object the `RefCell` is.
- **No check that can't fail**: what `vec![..]` runs is a move.

## Alternatives

- **Read a handle's `value` in place in tidy**, as `read_handles` does of
  a field's: the JS can't tell whether the index is written meanwhile,
  which MIR's writes say.
- **Treat every intrinsic as running nothing**: `catch_unwind`'s are
  function pointers it calls.
- **Name a `vec!`'s intrinsic alone**: one more name; the rule of what an
  intrinsic is given covers it and the next.

## Consequences

- Twenty-two corpus cases' JS is shorter: guards a temporary held read in
  place, `RefCell`s whose only counted borrow was across a `vec![..]`
  unchecked (`refcell_transparent`, `refcell_eq_through_guard`,
  `nullable_items`), and values written where they're assigned,
  `slot.value = $some(item)`, `chunk[0] = (chunk[$at(chunk, 0)] * 2) | 0`.
- A `&mut` to an element whose index a later argument of the same call
  writes, `add_to(&mut v[i], { i += 1; 10 })`, is a handle whose copy-back
  throws, as it did before: #280, found here.
- The function pointer case of an intrinsic is `catch_unwind`'s, which
  only std's own code calls, so no program rust-js compiles reaches it,
  and no mutation holds it.

## What holds it

- Corpus `refcell_vec_element_places` against native Rust: an element of
  a field's `RefCell<Vec<u32>>` through a temporary's guard and a `let`'s,
  `match &mut v[i]` and `v[i].as_mut()` of `Option`s with a `vec![..]` in
  an arm, a `&mut` to an element whose index is written after it's made,
  a reference reassigned in the index, an index out of bounds of each,
  and a borrow that meets another, which panics.
- Corpus `place_through_reassigned_reference`: a temporary `&mut`'s place
  through a reference reassigned while it's borrowed, and places whose
  index the value written writes, by a block and by a closure.
- Mutations in `scripts/mutations/lower/mir/queries.ts`
  (`index-parameter-not-given-once`), `lower/mir/locals.ts`
  (`guard-deref-writes-guard`), `lower/mir/assign.ts`
  (`in-place-not-held`, `settled-target-value-spilled`) and
  `lower/analysis/cell_borrows.ts`
  (`intrinsic-runs-anything`). A parameter written in the body isn't
  given once, but no mutation of that holds: `index_mut`'s index of a
  variable that doesn't read the same is held where it's borrowed,
  `const index$1 = index` (ADR 0881), before `same_place` asks of it.
