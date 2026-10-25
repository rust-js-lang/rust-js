# 0863. A `RefCell` of an object is the object, and one no borrow of which can meet another is never checked

Status: Accepted. Amends [0328](0328-refcell-borrow-counts.md) (what a
`RefCell` is, and which of its borrows are counted),
[0362](0362-refcell-fields-never-counted.md) (a field's plain cell, a
library's too), [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)
(what std's code a call runs may borrow),
[0582](0582-ptr-eq-of-an-object-is-its-identity.md) (`Rc::ptr_eq` of an
`Rc<RefCell<T>>`), [0585](0585-a-refcells-replace-holds-it-for-a-moment.md)
(`replace`, `take` and `get_mut` of an object) and
[0772](0772-a-nullable-cell-field-writes-null.md) (a nullable `RefCell`
field).

Case ([0262](0262-when-rust-and-js-disagree.md)): A, decided by the user
(2026-10-25). A `RefCell` of an object being the object changes no run:
the cell's every reader and writer reaches the one object, as through a
`&mut` (ADR 0147), and its count, where one is kept, is the object's own.
A borrow left unchecked is one that can't fail, so leaving out its check
changes no run either. Where a borrow may meet another, it's counted and
checked, and the program panics `already borrowed` where Rust's does:
Rust's behavior preserved. Where a library and its consumer would
disagree on a cell, the consumer is refused: D.

## Context

React's `ReactStartTransition.js` keeps a transition's types in an array
it shares, and a set of the fibers it updated:

```js
currentTransition.types =
  prevTransition !== null ? prevTransition.types : null;
..
if (prevTransition !== null && currentTransition.types !== null) {
  prevTransition.types = currentTransition.types;
}
..
currentTransition._updatedFibers = new Set();
```

Rust shares the array by `Rc` and changes it through a `RefCell`:
`types: RefCell<Option<Rc<RefCell<Vec<String>>>>>`. Every
`RefCell` was a `{ value }` box, every borrow counted and checked:

```js
const borrow = $borrow(transition.types, true);
let transitionTypes;
try {
  transitionTypes = borrow.value == null ? borrow.value : $rcClone(borrow.value);
} catch (error) {
  $unborrow(borrow);
  throw error;
}
..
replace = $cellReplace($borrowMut(transition.types), { value: [type] });
..
!$iter($lent($borrow(transitionTypes))).any((t) => t === type)
```

The array was `{ value: [...] }`, its `Rc` counted because `Rc::ptr_eq`
read it (`$rcClone`/`$rcDrop`), a nullable `RefCell` field and a
`*t.types.get_mut() = v` of what has a destructor were refused, and an
`Option<RefCell<HashSet<_>>>` was a box of a `Set`.

## Decision

**A `RefCell<T>` whose `T` is a JS object, a `Vec`'s or `VecDeque`'s
array, a map's `Map`, a set's `Set`, a struct's object or a tuple struct's
array, is that object: no `{ value }`.** Its borrows are the object, a
whole write changes it in place, and where its borrows are counted, the
object counts them (a non-enumerable `borrows`, as a `{ value }`'s was).

```rust
let types = transition.types.borrow().clone();
match types {
    None => {
        transition.types.replace(Some(Rc::new(RefCell::new(vec![ty.to_string()]))));
    }
    Some(types) => {
        if !types.borrow().iter().any(|t| t == ty) {
            types.borrow_mut().push(ty.to_string());
        }
    }
}
```

```js
const types = transition.types;
if (types == null) {
  transition.types = [ty];
} else if (!types.some((t) => t === ty)) {
  types.push(ty);
}
```

- **A whole write changes the object in place**, as a `&mut`'s does (ADR
  0147): `cell.replace(v)` and `cell.take()` are `$take(cell, v)`,
  `*cell.borrow_mut() = v` and `*cell.get_mut() = v` `$assign(cell, v)`,
  `swap` exchanges two objects' contents. A field's plain cell (ADR 0362)
  is the property, so its write is an assignment, `t.types = v`.
- **Not every object**: a reference, which a write points elsewhere; a
  class's instance, a binding's JS object, a struct of markers alone or
  one that's its field (ADR 0584) are `{ value }` still, as is a number,
  a string or an `Option`. A `Copy` struct is copied where it's changed in
  place, as any is (ADR 0623).
- **Not where generic code has a `RefCell<T>` of a type parameter's
  value** (in a body, a type's field or a trait impl's self type), which
  may be any `T`: every `RefCell` of an object stays `{ value }` then, so
  generic code reads one representation. Nor one made a `RefCell<dyn T>`
  or of a slice, whose cell is shared with what it's made of, nor one
  that may be one of those, to a fixed point.

**A `RefCell` type no borrow of which can meet another is never checked.**
A borrow is counted where it's held while code that may borrow the same
cell runs (ADR 0328's momentary borrow, now through a `match` arm's and an
`if let`'s scrutinee too), and where std's code holds it while the crate's
runs: a closure given to std, a `Clone`, `PartialEq`, `PartialOrd`, `Ord`
or `Debug` of the crate's that std's impl runs, a `Default` a `take`
runs; a cell's own impl, `a == b` of two `RefCell`s, borrows both. **A
guard a `let` holds**, `let v = log.borrow();`, is held until its block
ends, or until a statement of it drops it, `drop(v);`: where nothing in
between can ask, it's not counted either. One moved anywhere else,
`kept = v;`, may outlive its block, and is. What runs a destructor while a
guard is held asks what that destructor may: a value made, a `let` of one
moved from a place, `let q = p;`, which drops where its block ends, and a
write over one, `p = q;`. A type of which no borrow is counted, none of a
type that may be its cell (a `RefCell<dyn T>`'s, a generic
`RefCell<T>`'s), and whose own impls borrow nothing that may be it, is
**quiet**: its borrows are the object, `$borrow` and `$unborrow` gone,
and its guards drop nothing.

```rust
fn total(log: &Rc<RefCell<Vec<i32>>>) -> (i32, usize) {
    let v = log.borrow();
    let mut total = 0;
    for x in v.iter() {
        total += *x;
    }
    (total, v.len())
}
```

```js
function total(log) {
  const v = log;
  let total = 0;
  for (const x of v) {
    total = (total + x) | 0;
  }
  return [total, v.length];
}
```

```js
const seen = inner.seen;
if (seen != null) {
  seen.add(3);
  const count = seen.size;
  seen.clear();
}
```

Each cell that may meet another keeps its check, and panics where Rust
does: a borrow held while the crate's `Clone` that a `Vec`'s clone runs
borrows it mutably, while a closure given to std borrows it, while an `if
let`'s body borrows the cell its scrutinee holds, through a
`RefCell<dyn Shape>` that's also a `RefCell<Circle>`.

**A library's contract says what it does with its `RefCell`s** (the
manifest's `ref_cells`): those of an object it boxes, those it counts,
those it borrows unchecked, those its fields hold as their values, and
those it shares with consumers. Its consumer agrees, or is refused:

- a type a library shares as the object, which the consumer's generic
  code or `dyn` would box: refused, naming the library;
- a borrow held while code rust-js can't see runs, which may be the
  library's, of a type the library borrows unchecked: refused, "end the
  borrow before that code runs";
- a library's plain field (ADR 0362) the consumer lends, binds or holds:
  refused, "borrow it for a moment, or replace it".

A consumer counts a type its library counts, and reads a library's plain
field as the property.

**A nullable `RefCell<Option<T>>` field** (ADR 0772) is a plain field's
property: its `None` is `null` where it's made and written, through
`replace` and `get_mut` too. One that's a cell of its own is refused.

**`Rc::ptr_eq` of an `Rc<RefCell<T>>` of an object** is its identity,
`a === b`, uncounted (ADR 0582): the cell is the object, so two `Rc`s of
one are one JS value.

## Why

- **It's the JS React writes**, and any person: an array shared and
  pushed to, a `Set` cleared, a field assigned.
- **The check it drops can't fail**: a borrow that ends before any code
  that could borrow the cell runs meets no other, so its count is 0
  whenever it's checked.
- **One representation per type**, crate-wide and across libraries, so
  generic code, `dyn`, and a library's code read each cell alike.

## Alternatives

- **Keep `{ value }` and count every borrow**: React's port unlike
  React, for checks that can't fail.
- **The object only where no borrow is counted**: two representations of
  one kind of cell, the counted one boxed, and a type's representation
  changing with how a far body borrows it.
- **Count on a side table** (`WeakMap`): the object is the cell either
  way; a property on it is cheaper and is what `{ value }` did.

## Consequences

- The corpus's and the guide's `RefCell`s of arrays, maps, sets and
  structs are those objects; their borrows unchecked where none can meet
  another.
- `$withBorrow`, `$withBorrowMut`, `$showBorrowed`, `$replaceWith` and
  `$refCellSwap` take `itself` for a cell that's the object; `$eq` checks
  the cells it's given only where they're cells.
- `copied()`/`cloned()` of an iterator of `&&T` clones the `&T`, not `T`:
  it cloned the struct before, a wrong value of a `Clone` that differs.
- A library compiled before this has no `ref_cells`, read as empty.

## What holds it

- Corpus: `refcell_objects`, `refcell_value_copies`, `refcell_quiet_borrows`,
  `refcell_field_cells`, `refcell_identity_set`, `refcell_const_object`,
  `refcell_eq_through_guard`, `refcell_plain_field_drops`, and those that
  must panic `already borrowed`: `refcell_borrow_in_std_closure`,
  `refcell_borrow_in_std_clone`, `refcell_clone_borrows_itself`,
  `refcell_dyn_alias_borrow`, `refcell_if_let_held`,
  `refcell_bound_field_held`, `refcell_eq_while_mut`. Of a `let`'s guard:
  `refcell_let_guard_quiet`, and `refcell_let_guard_meets`,
  `refcell_let_guard_kept`, `refcell_let_guard_drops_moved` and
  `refcell_let_guard_writes_over`, which must panic.
- `test/crates.test.ts`, "a library's RefCells are the values they hold,
  and its consumer agrees or is refused", of `test/crates/pairs/ref_cells`
  and `ref_cells_counted`.
- `test/lowering-nullable.test.ts`, "a nullable RefCell field's None is
  null where it's written"; `test/lowering-cells.test.ts`.
- Mutations in `scripts/mutations/lower/drops/ref_cells.ts`,
  `analysis/cell_borrows.ts`, `drops/facts.ts`, `std_types/cell.ts`,
  `std_impls.ts`, `library.ts`, `runtime/borrow.ts` and others.
