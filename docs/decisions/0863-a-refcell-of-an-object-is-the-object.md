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
field). Amended by [0867](0867-a-binding-that-runs-nothing-and-aggregate-error.md)
(a binding that says it runs nothing borrows nothing) and
[0868](0868-what-stds-impls-and-derives-run-is-what-their-bounds-say.md)
(what std's impls and derives run is what their bounds say); amended
(a `#[repr(transparent)]` struct's `RefCell`, and a thread-local
`RefCell`'s `set`, a field's plain one by its declaration, below).
Amended by [0897](0897-an-element-a-guard-reaches-is-its-place.md): a
guard a temporary holds is its object where it's read, as `&mut guard`
given to `deref_mut` writes only what it guards.

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

## Amendment: a `#[repr(transparent)]` struct's `RefCell` is the struct, and of one the cell is its field's

Case ([0262](0262-when-rust-and-js-disagree.md)): C. Within a crate
every reader agreed on the box, and no run changed; at the JS boundary,
an array JS made and handed in, the results did: it has no `value`. React's taint registry keeps each request's cleanup
queue, an array Flight's JS makes and pushes to:

```rust
#[repr(transparent)]
pub struct RequestCleanupQueue {
    pub values: RefCell<Vec<Unknown>>,
}

queue.values.borrow_mut().push(entry);
```

The struct is its field (ADR 0584), so its field's `RefCell` is the
struct; but `queue.values`, only borrowed for a moment, was a field's
plain cell (ADR 0362), whose type was then no object cell anywhere: every
`RefCell<Vec<Unknown>>` was `{ value }`, and the push `queue.value.push(entry)`,
where JS's array has no `value`. A `RefCell` of a transparent struct of an
array, `RefCell<Lines>`, was `{ value }` too, as `holds_itself` said no to
every transparent struct, and a derived `Default` of one was refused.

**A `#[repr(transparent)]` struct's `RefCell` field is a cell of its
own**, not a field's plain cell: there's no property to hold its value,
the struct is the cell. So a `RefCell` of an array there is the array,
`queue.push(entry)`, as JS makes it, in a library and its consumer alike.

**A `RefCell` of a `#[repr(transparent)]` struct is its field's answer**
(`holds_itself`): `RefCell<Lines>`, of `struct Lines(Vec<String>)`, is
the array, `RefCell<Meters>`, of `struct Meters(f64)`, `{ value }`. Of
the field's type as declared, the struct's own type parameters in it,
so a generic transparent struct is one or none, as any is.

**A derived `Default` of a transparent struct is its field's**, as a
`take` of one asks.

### What holds it

- Corpus: `refcell_transparent`, a named and a tuple struct's, through
  an `Rc`, a set of `&'static`s, a slice, a `take` and a `replace`, and
  a `RefCell` of a `Vec`'s and a number's newtype, against native Rust.
- `test/crates.test.ts`, "a library's RefCells are the values they hold,
  and its consumer agrees or is refused": `dep::Queue`, the array in the
  library and in its consumer.
- Mutations `transparent-refcell-field-plain`,
  `transparent-refcell-holds-nothing`,
  `transparent-refcell-of-a-number-object` (`drops/ref_cells.rs`) and
  `transparent-default-unsupported` (`std_impls.rs`).

## Amendment: a thread-local `RefCell`'s `set` is its `replace`

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results
changed (issue #249). `LocalKey<RefCell<T>>::set(v)` was a `Cell`'s
`set` of its `{ value }`, whichever the cell was: of one that's the array
it holds, `ports.value = [1, 0]`, a property of the array, and the next
read saw the old one, `index out of bounds`. Nor was it borrowed, so a
`set` while a borrow of the thread-local was held didn't panic, where
std's does.

**A thread-local `RefCell`'s `set` is its `replace`, what it held
dropped**, as std's is: borrowed mutably to do it where its borrows are
counted, `$borrowMut(ports)`; of one that's the object it holds, the
object written in place, `$assign(ports, [1, 0])`, as `*cell.borrow_mut()
= v` is, or, of what has a destructor, `$take` and the old dropped once
the new is in its place; of a `{ value }`, its value, `$cellReplace`
where it's counted. A `Cell`'s is as before.

A lazy thread-local (ADR 0037, amended by #252) set before its first
use is made of what's given, the object itself where its `RefCell` is
one, `late.initWith([8, 9])`, not a `{ value }` of it.

### What holds it

- Corpus: `thread_local_refcell_set`, an array's, a number's, one of
  what has a destructor and a lazy one set before its first use, against
  native Rust; and
  `thread_local_refcell_set_borrowed`, a `set` while a borrow is held,
  which must panic `already borrowed`.
- Mutations `local-refcell-set-of-a-box`, `local-refcell-set-unchecked`,
  `local-refcell-set-old-undropped` and `lazy-set-made-a-box`
  (`std_types/cell.rs`).

## Amendment: a body std's code runs of no args is not known (#240)

`instantiated` read a callee's summary with the args of the call that
runs it. std's code that runs the crate's bodies, a `for` loop's
`Iterator::next` of a generic impl of the crate's, gave no args, so the
impl's own parameters, `[T; N]`'s, stayed in the caller's summary. When
`entry` ran `main`, they were instantiated with `main`'s `[]`, and the
compiler panicked (rustc's `issue-70507.rs` and
`projection-cache-registers-obligations.rs`). Each body std's code is
given now carries the args its type says: a closure's `closure_args`, a
function item's own args, and identity args for a closure the body
makes, since its parameters are the body's own. An impl's method that no
type gives args to, whose summary holds a parameter, is not known, so a
borrow held across it is checked. Held by
`test/corpus/generic_iterator_std_runs.rs` and the mutation
`unargued-summary-instantiated`. That closures carry their args changes
no output today, since a generic function's borrows across them are
counted already, so no mutation holds it. The readability baseline's
`while (true)` share grows by 0.1 in 10,000 lines and its `return;` by
0.2: the new case's `for` loop and its `next`'s early `None`.

## Amendment: a field's `RefCell` is plain by its declaration, borrowed or not

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A field's `RefCell`
no body borrows has no borrow to check; one that's borrowed is decided
as before.

React's `Fiber` has `child`, `sibling`, `return` and `alternate`, each a
`#[rust_js::nullable]` `RefCell<Option<Rc<Fiber>>>`, which the
reconciler's other files read. Its `ReactInternalTypes`, which reads
none of them, was refused: "`#[rust_js::nullable]` of a `RefCell` is of
one a field holds as the value it holds .. this one is lent, bound,
held ..". A field's `RefCell` type was plain (ADR 0362) only where some
body borrowed a field of its type for a moment, so `child` passed once
`alternate`, of the same type, was read somewhere; and one never read
was `{ value }`, `child: any` in the declarations. What a type is in JS
hung on whether some function happened to borrow it.

**Each `RefCell` type a struct of the crate's has as a field's is plain
by that declaration**, as one borrowed only for a moment is: it's taken
away where a body lends one, binds one by a pattern or derives an impl
reading one, a generic one's, or one a library shares as a cell, as
before (`find_ref_cells`). A borrow of one held while code that may
borrow it runs is still counted and checked.

- **A nullable one's `None` is `null` where it's made**, as a nullable
  `Cell`'s is (ADR 0772): `Fiber { child: RefCell::new(None), .. }` is
  `{ child: null }`, where it was left out, as only an optional one is
  (ADR 0810). Corpus `refcell_clone_through_rc`'s `Internals { queue:
  RefCell::new(None) }` is `{ queue: null }`.

`test/lowering-nullable.test.ts`, "a nullable RefCell field no body
borrows is the value it holds", holds it, with mutations
`field-refcell-plain-by-use` (`lower/drops/ref_cells.ts`) and
`nullable-refcell-made-undefined` (`lower/aggregates.ts`).
