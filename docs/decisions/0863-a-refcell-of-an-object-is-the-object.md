# 0863. A `RefCell` of an object is the object, and one no borrow of which can meet another is never checked

Status: Accepted; amended below (`as_deref` of an `Option` of a guard, a guard that's only a call's receiver, a guard read only for a binding's argument (rd427), a held borrow counts the cells that can be its own (#445), a library's cell only its own code and constants reach is its flow's (#456), and a `RefCell` that's a box is declared `{ value: T }`, an enum variant's `RefCell` field holds its value by its declaration, and a pattern's binding by reference of a field is that field's own place (#503)). Amends [0328](0328-refcell-borrow-counts.md) (what a
`RefCell` is, and which of its borrows are counted),
[0362](0362-refcell-fields-never-counted.md) (a field's plain cell, a
library's too), [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)
(what std's code a call runs may borrow),
[0582](0582-ptr-eq-of-an-object-is-its-identity.md) (`Rc::ptr_eq` of an
`Rc<RefCell<T>>`), [0585](0585-a-refcells-replace-holds-it-for-a-moment.md)
(`replace`, `take` and `get_mut` of an object) and
[0772](0772-a-nullable-cell-field-writes-null.md) (a nullable `RefCell`
field). Extended by [1046](1046-an-untagged-enums-arrays-are-told-by-their-length.md):
a `RefCell` of an untagged enum each of whose values is an array it holds
itself is that array (rd490). Amended by [0867](0867-a-binding-that-runs-nothing-and-aggregate-error.md)
(a binding that says it runs nothing borrows nothing) and
[0868](0868-what-stds-impls-and-derives-run-is-what-their-bounds-say.md)
(what std's impls and derives run is what their bounds say); amended
(a `#[repr(transparent)]` struct's `RefCell`, and a thread-local
`RefCell`'s `set`, a field's plain one by its declaration, and each
place's by that place, a struct's field by its declaration, and a
thread-local and a field made where its struct is each its own, below).
Amended by [0897](0897-an-element-a-guard-reaches-is-its-place.md): a
guard a temporary holds is its object where it's read, as `&mut guard`
given to `deref_mut` writes only what it guards. Amended by
[1038](1038-a-trait-of-classes-method-that-gives-a-field-is-that-field.md):
a call of a trait of classes' method that gives a field is that field.

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

## Amendment: each place's `RefCell` is that place's, a struct's field by its declaration

Case ([0262](0262-when-rust-and-js-disagree.md)): C, the results
changed. React's root, `FiberRootNode`, is made by React's JS, and its
fibers by JS too; the port reads them. Which `RefCell`s were the value
they hold was decided by type, so one use of a type anywhere changed
every field of it:

```rust
pub struct Root { pub times: RefCell<Vec<f64>> }
pub fn mark(root: &Root, index: usize) { let mut times = root.times.borrow_mut(); times[index] = 1.0; }
pub struct Cursor<T> { pub current: RefCell<T> }
pub fn read<T: Clone>(cursor: &Cursor<T>) -> T { cursor.current.borrow().clone() }
```

`Cursor`'s `RefCell<T>`, a cell of any value, boxed every `RefCell` of an
object, and `root.times`, held by a `let`, wasn't a plain field: `mark`
read `times.value[..]` of the array JS made, `undefined`, and React's
code threw. And a `thread_local!` of a `Fiber`'s field's type,
`RefCell<Option<Rc<Fiber>>>`, lent its type to std's `EagerStorage` field
inside the macro, which made every field of it a cell: `fiber.return.value`,
or, `#[rust_js::nullable]`, refused.

**Each place's `RefCell` is decided by that place.**

- **A struct's field holds its value by its declaration** (not a
  `#[repr(transparent)]` struct's, which is its field, the cell): the
  property is what the cell holds, a number, an `Option`, a fiber, the
  array, whatever else has its type and however it's used. Of an object
  the field holds the object, which is the cell (`is_value_cell` of the
  type it's declared, so that each of a generic struct's is one or none).
  A library's field the same, read of its declaration on both sides.
- **A field none of whose borrows can meet another**, never lent, bound or
  moved, and never borrowed while code that may borrow it runs, is never
  checked, whatever its type's borrows elsewhere: its borrows are the
  property, `times[..]`, its guards release nothing (`quiet_field`, MIR's
  `QuietCells`). A library says each of its own in its contract, as a
  fact of a set (ADR 0906): `fields: [{ key, facts: ["quiet"] }]`, read
  by `FieldFact::Quiet`, library ABI 7; a consumer that lends one, or holds a borrow of one while code that may
  borrow it runs, is refused, naming it. A derive rust-js writes itself,
  `Clone`, `PartialEq`, `Debug`, reads each `RefCell` field through the
  `RefCell`'s own impl, which holds a borrow while what it holds runs its
  own: of a type whose impls may borrow it, that field may meet another,
  and is counted. (A `#[derive(Clone)]` has no body lowered, so before this
  its `RefCell` field was a plain one whose clone's borrow went unseen, and
  a `Clone` of what it holds that borrowed it mutably ran on where std's
  panics: `refcell_field_derive_meets`.)
- **Where a field's cell moves to another representation, the field's
  owner converts it there**, naming the place in the JS. One lent, `&x.f`,
  bound by a pattern, or borrowed while code that may borrow it runs, is
  its one cell, `$fieldCell(x, "f")`, the same cell wherever it's asked,
  kept beside the object JS made, its `value` the property: what it
  counts is the field's. A nullable one's, `$fieldCell(x, "f", true)`,
  writes its `None` `null`. A borrow of one where nothing's counted reads
  the property, `$fieldCell(x, "f").value` being `x.f`. One moved out
  whole, `let Wrap { n } = wrap;`, is a cell of its own of what it held,
  `{ value: wrap.n }`, which the field written again doesn't change; one
  written whole, `wrap.n = RefCell::new(9)`, is the property written,
  `wrap.n = 9`, as a `Cell` field's now is too (it crashed the printer).
- **A generic struct's `RefCell<T>` field doesn't box another place's.**
  Its generic code reads it as a box of the property, whatever `T` is, so
  it's no `RefCell<T>` of any value: only where some borrow of it may meet
  another is each `RefCell` of an object it's made of, `Cursor<Vec<u8>>`'s
  `RefCell<Vec<u8>>`, a box, so its generic code's `$fieldCell` and its
  concrete code's read agree; of one none can, the object, which nothing
  counts.
- **A thread-local by its own uses** (ADR 0270): one that's its module's
  variable, `quiet_locals`, is set as the variable, `current = fiber`, its
  borrows never counted, whatever its type's are elsewhere. A local's
  cell, ADR 0287's, by its own too.

```js
export function mark(root, index) {
  const times = root.times;
  times[$at(times, index)] = 1;
}

export function read(cursor, TClone) {
  const borrow = $borrow($fieldCell(cursor, "current"), true);
  try {
    return TClone.clone(borrow.value);
  } finally {
    $unborrow(borrow);
  }
}

export let current;

export function setCurrentFiber(fiber) {
  current = fiber;
}
```

What stays a type's, as every place of it reads it so: whether a cell
that isn't a struct's field, a `&RefCell<T>`'s, an `Rc`'s, a `Vec`'s
item's, is the object or `{ value }` (one boxed reaches generic code and
`dyn`), and whether a borrow of a type that may be lent is counted, as a
guard's drop asks of its type.

### Why

- **JS made the object, and the declaration is its shape**, the one its
  `.d.ts` says, `times: number[]`, `return?: Fiber | null`: a far body's
  use can't change what JS must write.
- **A conversion at the point it's needed is visible and correct**: the
  field's one cell counts every borrow of it, from each place that has it,
  so a borrow held while another's taken panics `already borrowed` as
  std's does.
- **Refusing every lent field** would refuse what compiled before, the
  corpus's `refcell_bound_field_held` and `refcell_field_cells`; `{ value }`
  in the field was the wrong answer.

### Alternatives

- **A field a cell where its own uses lend it** (the field's own uses,
  not its type's): no other place changes it, but its own use still
  changes its JS shape silently, a JS-made object's field read as a box.
- **A count on the owner object**, a hidden `borrows` per key: as a stable
  cell does, but every borrow helper would take an object and a key.

### What holds it

- Corpus: `refcell_field_places`, the root, the cursor, the fiber and its
  thread-local, a field lent, borrowed, moved out and written whole,
  against native Rust; and those that must panic `already borrowed`:
  `refcell_field_lent_held`, a field lent while it's borrowed again,
  `refcell_generic_field_held`, a generic field made of an array, and
  `refcell_field_derive_meets`, a derived `Clone`'s; and
  `refcell_bound_field_held`, `refcell_field_cells`,
  `refcell_let_guard_quiet` and `rc_counts` as they are now.
- `test/lowering-field-cells.test.ts`, "a struct's RefCell field holds
  its value whatever else has its type" and "a struct's RefCell field lent
  is its one cell, of the object JS made", given objects JS made, one of
  which panics `already borrowed`; and "a transparent struct's nullable
  RefCell field is refused".
- `test/lowering-nullable.test.ts`, "a nullable RefCell field's None is
  null where it's written": one lent is `$fieldCell(slot, "held", true)`.
- `test/crates.test.ts`, "a library's RefCells are the values they hold,
  and its consumer agrees or is refused": its quiet `fields`, one read as
  the property by a consumer that counts borrows of its type, and a lend
  of one refused naming `dep::Transition::types`.
- Mutations in `scripts/mutations/lower/drops/ref_cells.ts`,
  `representation.ts`, `mir/local_facts.ts`, `std_types/cell.ts` and
  `runtime/borrow.ts`.

## Amendment: a thread-local is a place of its own, and a field made where its struct is

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same run; what
was refused compiles. The `{ value }` a generic `RefCell<T>` is stays
the generic value's.

React's `ReactFiberStack` keeps its stack in a thread-local,
`VALUE_STACK: RefCell<Vec<Option<&Unknown>>>`, beside a generic
`create_cursor<T>` that makes a `StackCursor { current: RefCell::new(v) }`;
`shared`, which the reconciler uses, hands it the same `RefCell` type as
the array, its internals' `thrownErrors`. The reconciler was refused:
"this crate has `RefCell<Vec<Option<&Unknown>>>` as a `{ value }`, of its
generic code's `RefCell<T>` of any `T` .. but `shared` .. shares one with
it as the object it holds". Two things stood for a value of that type that
no code has:

- **The thread-local's own storage**: the closure its macro makes of its
  key, `EagerStorage<RefCell<..>>`, and the key, `VALUE_STACK` of
  `VALUE_STACK.with_borrow_mut(..)`, `LocalKey<RefCell<..>>`, and what its
  `init` gives, `RefCell::new(vec![..])`. A thread-local is a place, as a
  struct's field is: its module's variable, the array, where only its
  module reads and sets it (ADR 0270), and its cell's type's where it's
  lent, by what it's lent to, `with(|cell| f(cell))`'s closure.
- **A field's `RefCell` made where its struct is made**, `current:
  RefCell::new(default_value)`, which was a `RefCell<T>` of any value:
  every `RefCell` of an object the crate has was then `{ value }`. The
  field holds its value by its declaration (above), so the cell it's made
  of is the field's, as a read of it was already.

**A thread-local's key, its storage and what its `init` gives, and the
`RefCell` a struct's field is made of where the struct is, are those
places' own, no value of their cell's type**: neither what the crate is
said to have of a type, which a library must agree with, nor a `RefCell<T>`
of any value. A generic `RefCell<T>` made and given, `cell_of<T>(v) ->
RefCell<T>`, is still `{ value }`, and every `RefCell` of an object with
it, so generic code reads one representation.

```rust
thread_local! {
    static VALUE_STACK: RefCell<Vec<Option<&'static Unknown>>> = const { RefCell::new(Vec::new()) };
}
pub fn cell_of<T>(value: T) -> RefCell<T> { RefCell::new(value) }
pub fn clear() { VALUE_STACK.with_borrow_mut(|value_stack| value_stack[0] = None); }
```

```js
const valueStack = [];

export function cellOf(value) {
  return { value };
}

export function clear() {
  valueStack[$at(valueStack, 0)] = undefined;
}
```

A crate with no generic `RefCell<T>` value but a generic struct's field
made so now has its other `RefCell`s of objects as those objects: corpus
`refcell_generic_field_made`'s `log` is `[4, 5]`, `{ value: [4, 5] }`
before. Its `c.current.replace(vec![8])` found a wrong answer this change
uncovered: a quiet field's `RefCell` of an object replaced through a
reference to it, a variable, assigned the variable, `replaced = [8]`, not
the field. Through a reference it's replaced in place, `$take(c.current,
[8])`; at the field's place, `c.current`, the property is written, as
before.

### What holds it

- `test/crates.test.ts`, "a thread-local's RefCell is its own, whatever
  generic code's RefCell<T> is", of `test/crates/pairs/thread_local_cell`,
  against native Rust: a `const` thread-local and one made by its `init`,
  each the array, beside a generic `RefCell<T>`'s `{ value }`.
- Corpus `refcell_generic_field_made`, against native Rust.
- Mutations `thread-local-storage-a-value`, `thread-local-key-a-value`,
  `thread-local-init-a-value`, `field-made-of-any-value`
  (`lower/drops/ref_cells.ts`) and `quiet-replace-through-a-reference`
  (`lower/std_types/cell.ts`).

## Amendment: a let-else's guard is dropped before its `else`

React walks to the last update, `let Some(next) =
update.next.borrow().clone() else { break };`. The guard is the
initializer's temporary, which Rust drops before the `else` runs, in
either edition, as an `if let`'s is before its `else` (edition 2024); the
JS released it there, `$unborrow(borrow)` before `break`. But the drop
facts gave a let-else's statement no extent for its temporaries
(`statements` in `drops/facts.rs`), so its borrow wasn't momentary: it
was counted, and so was every borrow of `Update::next` crate-wide,
`$cellReplace($borrowMut($fieldCell(update, "next", true)), next)`
where it was `update.next = next`.

**A let-else's temporaries live while its initializer runs**, as a
`let`'s do: its guard is momentary where nothing the initializer runs
may ask of its cell, and the field stays its value, by its place's rule
above. One the initializer's code may meet, `cell.borrow().map(|v| (v,
cell.try_borrow_mut().is_ok()))`, is counted as before.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): Rust's drop, and no
check where none can fail.

Held by `test/corpus/let_else_guard_released.rs`, against native Rust,
and the mutation `let-else-guard-held-through-else`.

## Amendment: a cell's own impls are only those it has

React's thenables keep their listeners, `Rc<RefCell<Vec<&dyn Fn()>>>`,
each borrow a moment's: `listeners.borrow_mut().push(listener)`, and
`while i < listeners.borrow().len() { let listener =
listeners.borrow()[i]; listener(); i += 1 }`, no borrow held while a
listener runs. Upstream's are an array. But each borrow was checked,
`$borrowMut(listeners, true)` in a `try`/`finally`, and the loop a
`while (true)` testing a borrow: the type wasn't quiet, as its own impls,
`Clone`, `PartialEq`, `PartialOrd`, `Ord` and `Debug`, were each walked
for what they may run (`CellBorrows::impls_ask`), and the walk of one
whose impl std's bounds didn't say, `Vec<T>: PartialEq<Vec<U>>`, reached
the `dyn Fn()`, which may run anything.

**Only an impl the cell has may run**: `RefCell<Vec<&dyn Fn()>>` has no
`PartialEq` nor `Debug`, as no `dyn Fn()` has, so neither runs, nor
anything of the `dyn Fn()`'s; its `Clone` copies the references. rustc
says which it has, `codegen_select_candidate`'s `Unimplemented`. Of a
type parameter's cell, `RefCell<Vec<T>>` in a generic body, which may be
any, rustc can't say, and each may: a generic `flip<T>(c: &RefCell<Vec<T>>)`
borrowing its cell mutably from a `Node`'s `Clone`, while the cell's own
`clone` holds it, still panics `already borrowed`.

Case A ([0262](0262-when-rust-and-js-disagree.md)): a borrow left
unchecked is one that can't fail. Held by
`test/corpus/refcell_listeners_momentary.rs`, against native Rust, a
listener adding another while they run, and
`test/corpus/refcell_generic_borrow_meets_held.rs`, the generic body's
panic; the mutations `impls-ask-of-what-it-hasnt` and
`impls-had-of-a-parameter-unknown` (`analysis/cell_borrows.ts`).

## Amendment: a `const` of a struct with a `RefCell` field is its initializer's

A `const` of a struct whose `RefCell<Option<u8>>` field is the value it
holds, by its declaration, was written from rustc's value of it as the
cell's `{ value }`, `{ items: {} }`: a read of the field, `slot.items !=
null`, took its `None` for `Some`, a wrong answer.

**A struct's `RefCell` field may be the value it holds only as the
crate's bodies decide, so a constant of a struct with one is its
initializer's**, lowered as a body is, as a `RefCell` of an object's
already was: `const EMPTY = { count: 0 };`, its `None` the field left out.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)):
Rust's answer, preserved. Tested by `test/corpus/ref_cell_field_const.rs`
against native Rust; its mutation is `ref-cell-field-const-boxed`.

## Amendment: a cell's `dyn` is any type only where made in place (ADR 0960)

`may_be_cell` took a `dyn` anywhere in a cell's type for any type, so a
`RefCell<Vec<Rc<dyn Fn()>>>` borrowed while its hooks run checked every
`RefCell<Vec<Rc<Node>>>` (#374). A `dyn` is any type only where a value is
made one in place, the cell's own value or a struct's last field; elsewhere
it's a `dyn` of its trait. See [0960](0960-a-dyn-is-any-type-only-where-made-in-place.md).

## Amendment: a borrow only where the loop's item isn't found meets none

React's scheduleGesture merges a transition's types into its gesture's,
a port of upstream's loop: `for transition_type in
transition_types.borrow().iter() { if
!scheduled_types.borrow().contains(transition_type) {
scheduled_types.borrow_mut().push(transition_type.clone()); } }`, both
`Rc<RefCell<TransitionTypes>>`, `react_isomorphic`'s type, whose borrows
that library checks nowhere. It was refused: "this borrow .. is held
while code rust-js can't see runs". Two answers were wrong:

- **A cell's borrow was code rust-js can't see.** `call_runs_unseen`
  asked `called`, which needs the borrow's receiver to say which cell it
  borrows, and given none answered `Any`. Which cell is the question of
  what a borrow may meet, not of what code it runs: a `RefCell`'s
  `borrow`, `borrow_mut` and `try_borrow*`, and a lock's, run nothing but
  their own check. `call_runs_unseen` now says so first.
- **The push's borrow was counted as meeting the loop's**, the two of
  one type. It can't: were `scheduled_types` the loop's cell,
  `transition_type`, one of its items, would be in it, and `contains`
  would find it, a `String` being equal to itself; the `then` never runs.

**A borrow of a variable's cell inside `if !v.borrow().contains(t)`
meets no borrow `own` holds** (`Quiet::unmet_in`) where:

- `t` is the variable a `for` over `own`'s guard's items binds, its head
  `own`'s guard read by reference, through `Deref` of an `Rc`, a `Ref`
  or a `Vec` and a slice's `iter()`, never a type of the program's,
  whose `deref` may give another place each time (`items_of_own`);
- what's compared is equal to itself: a string's, an integer's, a
  `bool`'s or a `char`'s, not a float's, `NaN`;
- `v` is a handle the body never sets again nor lends mutably, so one
  cell wherever it's read.

A borrow of `v` in the `then` is then no borrow that may ask of `own`;
any other borrow there is asked as before. Case N
([0262](0262-when-rust-and-js-disagree.md)): where the cells differ, the
borrows can't meet; where they're one, the `then` never runs; so no
check fails either way, in Rust or JS.

```js
for (const transitionType of transitionTypes) {
  if (!scheduledTypes.includes(transitionType)) {
    scheduledTypes.push(transitionType);
  }
}
```

The port's ReactFiberGestureScheduler compiles, its merge upstream's,
and no other module of the reconciler checks its types' borrows.

`docs/readability.txt`'s `while (true)` share grows 10 to 10.2 in 10,000
lines, and `return;`'s 10.3 to 10.5: the three cases where the borrows
would meet are counted loops, `$next` and `$unborrow` in a `while
(true)`, each returning as it unborrows, as Rust checks them.

Held by `test/corpus/contains_guarded_borrow.rs` against native Rust,
uncounted, and its three where the borrow would meet, each panicking as
Rust does, counted: `contains_guarded_float.rs`,
`contains_guarded_other.rs`, `contains_guarded_set_again.rs`;
`test/crates.test.ts`'s "a library's RefCells are the values they hold",
a consumer's borrow held while it borrows another counted, not refused,
and the merge of the library's cells uncounted; and the mutations
`cell-borrow-runs-unseen` (`analysis/cell_borrows.ts`),
`unmet-of-a-float`, `unmet-of-any-value`, `unmet-never` and
`unmet-of-a-handle-set-again` (`drops/facts.ts`).


## Amendment: a library's thread-local, and an assignment's value

Case ([0262](0262-when-rust-and-js-disagree.md)): none; no borrow these
leave unchecked can meet another, as before.

Two refusals of React DOM's port, of a crate built against React's
libraries, which borrow their cells unchecked (gaps rd76, rd32):

- **A consumer's `with_borrow` of a library's thread-local `RefCell`**,
  `CURRENT.with_borrow(|current| current.clone())` of the reconciler's
  `ReactCurrentFiber`, was counted, as only the crate's own keys could be
  quiet (`quiet_locals`), and refused as "held while code rust-js can't
  see runs". **One every lend of which in the crate, a `with_borrow`'s or
  a `with_borrow_mut`'s, runs what borrows nothing that may be its cell,
  as the crate's own are asked (`plain_locals::quiet_library_keys`), isn't
  counted**: `return current;`, upstream's `const child = current;`. One a
  `with` lends, or whose closure runs the library's code, which may borrow
  it, is counted as before, and refused where its library borrows it
  unchecked.
- **A guard of an assignment's place**, `*current.borrow_mut() =
  js::concat_text(value);`, was taken as held while the whole statement
  runs, its value too, a binding's or a library's call; Rust evaluates
  the value first ("the assigned value operand is evaluated first"), so
  **the guard is held while the place is read and written alone**, and
  of a place whose old value has a destructor, which runs then, the whole
  statement, as before (`drops/facts.rs`'s `momentary`).

Held by `test/crates.test.ts`' two-crate cases `thread_local_ref_cell_read`
and `assigned_after_its_value`, against native Rust;
`test/cross-crate-items.test.ts`' "a library's thread-local is borrowed
quietly only where nothing it runs may borrow it", whose `with_borrow`
calling the library's `push`, which `with` lends its cell, is refused; and
mutations `quiet-library-key-counted` (`drops/ref_cells.rs`),
`assigned-value-held` (`drops/facts.rs`) and `quiet-library-key-unasked`
(`analysis/plain_locals.rs`).

## Amendment: `as_deref` of an `Option` of a guard

clippy's `option_as_ref_deref` asks for `types.as_deref()` of an
`Option<Ref<'_, Vec<String>>>` where a port wrote `types.as_ref().map(|t|
t.as_slice())`, which rust-js refused ("`as_deref` of a `Ref`").

**`as_deref` of an `Option` or a `Result` of a `RefCell`'s guard is what
the guard derefs to, `guard_value`'s** (`combinators/values.rs`'s
`deref_value`): of a cell of an object, the object, so the subject as it
is, read in place, `types?.length`, as an uncounted `Rc`'s is; of a
number's cell, its `value`, `count?.value`. It borrows the guard, as
Rust's does; the guard is given back where Rust drops it.

Case (ADR 0262): N, no difference: the value is what Rust's `&*guard`
reads.

Held by `test/corpus/option_of_a_ref_as_deref.rs` (against native Rust);
mutations `as-deref-of-a-guard-refused` (`combinators/values.ts`) and
`as-deref-of-an-object-guard-tested` (`combinators.ts`).

## Amendment: a trait of classes' method that gives a field (rd304)

A call of an impl's method of a trait of classes that only gives a
reference to a field of its `self` is that field: its own `&self.f`
lends nothing, a borrow through its call is the field's, and its call
held elsewhere lends the field. A default's call of a trait of classes'
method, of a `Self` not known, runs what one of its classes' does
([ADR 1038](1038-a-trait-of-classes-method-that-gives-a-field-is-that-field.md)).

## Amendment: a guard that's only a call's receiver (rd360)

Case ([0262](0262-when-rust-and-js-disagree.md)): B, the user's decision
(2026-11-01). React DOM's Fizz config pushes each attribute's chunks while
it escapes the attribute's value, a prop, any value:

```rust
target.borrow_mut().extend([
    chunk(&ATTRIBUTE_SEPARATOR),
    string_to_chunk(name),
    chunk(&ATTRIBUTE_ASSIGN),
    string_to_chunk(&escape_text_for_browser(Some(value))),
    chunk(&ATTRIBUTE_END),
]);
```

Rust takes the guard before it evaluates the arguments, and a `toString`
of the value, JS's, may call back and push to `target`, which Rust would
meet with a panic. So the guard was held while code rust-js can't see
runs, and with it every chunk array's borrow in the crate was counted:
122 `$borrowMut`s in `try`/`finally`. Upstream's `target.push(..,
stringToChunk(escapeTextForBrowser(value)), ..)` evaluates its arguments
first.

**A guard that's only the receiver of its statement's call is held while
the call runs, not while its other arguments do** (`drops/facts.rs`'s
`held_quietly_after`): `target.borrow_mut().push(show(value));` is
`target.push(show(value))`, which runs `show` first. Its receiver, and
the call itself, are asked as before. Of its other arguments, only what
rust-js sees asks of the guard (`CellBorrows::call_asks_seen`): a borrow of
the cell the program wrote, `c.borrow_mut().push(c.borrow().len())`, or one
a body of the crate's that an argument calls makes, is still counted and
panics as Rust's does. Code that may call back, a binding's or a
library's, doesn't.

Only an argument whose code rust-js can't see, which calls back at once
and borrows the receiver's cell, tells them apart: Rust panics `already
borrowed`, and the JS does what upstream's does, the callback's push
first.
Listed in ADR 0262's Consequences and the guide's differences
(`semantics.md`, 39).

Held by `test/lowering-cell-borrows.test.ts`'s "a guard that's only a
call's receiver is held while the call runs, not its arguments", and by
mutations `receiver-guard-held-through-arguments`,
`receiver-guard-ignores-arguments`, `receiver-guard-ignores-call` and
`receiver-guard-arguments-unseen` (`drops/facts.rs`), and
`seen-borrows-own-only` and `seen-asks-nothing` (`analysis/cell_borrows.rs`).
The corpus's `trie_of_counted_rcs` reads `cur.borrow().parent` uncounted.

## Amendment: a held borrow counts the cells that can be its own (#445)

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the user's decision
(2026-11-01). Leaving out a check that can't fail changes no run. A borrow
that can fail is still checked and panics where Rust's does.

A held borrow counted every cell of its type: `counted` held types, and
`quiet` asked `may_be_cell` of each. React DOM's `hydrateHoistable` holds a
tag cache's `nodes.borrow_mut()` across a prop's getter. The cache is a
`Rc<RefCell<Vec<&Element>>>` kept in a JS `Map`. Fizz's `Batch` is a
`RefCell` of the same type that JS gives, `RB`. So Fizz's loop over a batch
printed `$borrow(batch)[i]`, `$borrowMut(batch).length = 0` and
`$stepBy(..)`, where alone it printed `for (let i = 0; i < batch.length;
i += 2)` (rd201). No batch is ever a tag cache.

**A borrow is counted only where a cell it can be is borrowed while code
that may borrow that cell runs** (`analysis/cell_flow.rs`, `CellFlow`).
Which cells a borrow can be of is a value-flow of the crate's cells,
Steensgaard's unification over its bodies' MIR:

- **Each value is a node**, and what it's made of a node beneath it: a
  struct's or a tuple's field, a closure's capture, what a reference or a
  `Box` points at, an array's items, and what a std type or a JS object
  holds of each of its type parameters. A move, a copy, a reference, an
  aggregate, a call of a body of the crate's and its result make two nodes
  one, and what's beneath them with them.
- **A call of std's moves a `T` only where a `T` goes**, so each value of
  a type it's given or gives is one with each other of that type. That
  includes what a closure or impl std runs is given and gives, by
  `CellBorrows::std_runs`. Std's code can't reach inside the crate's own
  types: it moves one whole. So is a binding that keeps what it's given in
  its collection, a JS `Map`'s `get` and `set`
  ([1050](1050-a-js-collection-keeps-what-its-given-only-in-itself.md)).
- **What code rust-js can't see is given, or gives, is `TOP`**: a
  binding's call, a library's, a `dyn`'s, a fn pointer's, one rustc can't
  resolve, and std's code that may run any; a value made a `dyn`, a
  pointer's cast, a transmute, a static. So is what an exported function,
  a trait's method or a coroutine is given and gives. To a fixed point, so
  is what a function or closure whose value `TOP` reaches is given and
  gives. And so is a value of a type whose destructor isn't std's, which
  runs given it.
- **A cell's class is its node.** One `TOP` reaches may be any other such
  cell of its type, as every cell was before. One `TOP` doesn't reach is
  reached only along the flows unification followed.

`RefCells` keeps, beside the types counted, each counted borrow's class
(`counted_classes`), and the types of those `TOP` reaches
(`counted_reached`) or whose class isn't known (`counted_unknown`). A
borrow's THIR call is its MIR's by where it's written and what it calls,
where the counts of each agree. **A borrow of a class `TOP` doesn't reach
is quiet where none of its class's borrows is counted, and none whose
class isn't known may be it.** One `TOP` reaches is quiet where none of a
type that may be it that `TOP` reaches, or not known, is counted, nor one a
library counts. Either is quiet only if its own impls borrow nothing that
may be it (`quiet_class`). MIR's `QuietCells` marks such a borrow and its
guard, which releases nothing, as it marks a quiet field's
(`local_facts.rs`). What asks by type, a guard's drop by its type, a
field, a thread-local's own cell, is what it was.

A crate any of whose bodies has no MIR has no classes, and every cell is
counted by its type.

**A library's contract stays per type.** `RefCellFacts.counted` is each
type some borrow of which is counted, as before, and a consumer counts
each. A cell a library shares is one `TOP` reaches in both crates. A
consumer may hold a borrow of one while the library's code runs, and the
library has only its contract's types to say so. So in a library, a
borrow of a cell `TOP` reaches is quiet only where its type is, as
before. Only its own cells, which no consumer reaches, are each their
class. `unchecked` is, as before, each type quiet by its type that the
library borrows of a cell `TOP` reaches, or whose class isn't known. A
borrow of a cell only the library's code reaches is never a consumer's,
so its type isn't listed for it.
React's reconciler holds a `RefCell<Vec<Rc<Fiber>>>` of its own across
code. Were its shared cells of that type unchecked, React DOM, which holds
one across the reconciler's code, would be refused.

**Not a rule of where a cell was made.** "A cell in an `Rc` the crate
made is never a binding's" doesn't hold: `Rc::new(batch)` of a `Batch` JS
handed in moves JS's own array into the `Rc`. Flow follows the array, so
that `Rc`'s cell is one `TOP` reaches.

```js
export function reveal(batch) {
  let total = 0;
  for (let i = 0; i < batch.length; i += 2) {
    total = (total + batch[i]) >>> 0;
  }
  batch.length = 0;
  return total;
}
```

### Alternatives

- **Per module**: a cell given across modules is the same cell. Unsound.
- **Flow-sensitive, or per call site**: more precise, and much more to
  prove. Unification is linear, and its classes are what the port needs.

### What holds it

- `test/lowering-cell-flow.test.ts`: "a held borrow counts the cells that
  can be its own, not every cell of its type", the tag cache in a `Map`
  checked and panicking, the batch plain; "a cell JS gives may be any cell
  of its type JS reaches"; and "a cell given to JS may be any cell JS
  gives back".
- Corpus, against native Rust, each of which must panic: cases
  `refcell_flow_through_std` (a `Vec`, a `HashMap`, `get`, `cloned`, `map`
  and a closure), `refcell_flow_thread_local`,
  `refcell_flow_thread_local_init`, `refcell_flow_lent_key`,
  `refcell_flow_reaches_unseen` (a generic `f()`) and
  `refcell_flow_through_dyn`. Cells of the same type beside them that
  can't be the held one are plain. Snapshots `untagged_ref_cells` and
  `refcell_borrows` lose the checks they no longer need. The readability
  baseline counts the new cases. Over the cases it counted before,
  `refcell_borrows`' five fewer lines raise `$iter`'s share from 83.6 to
  83.7 in 10,000 lines, though no `$iter` was added.
- `test/crates.test.ts`: a consumer's own cell beside a library's held one
  is unchecked; "a library checks a cell its consumer may share as its
  type says", pair `ref_cells_flow`, against native Rust.
- Mutations: `flow-std-moves-nothing`, `flow-closure-arguments-apart`,
  `flow-key-init-apart`, `flow-binding-keeps-in-collection`,
  `flow-collection-unseen`, `flow-exports-closed`,
  `flow-given-functions-closed`, `flow-unresolved-call-closed` and
  `flow-destructor-closed` (`analysis/cell_flow.rs`); `class-own-uncounted`,
  `class-own-of-no-unknown`, `class-reached-uncounted`,
  `class-reached-ignores-library-counted`,
  `class-reached-precise-in-a-library` and `class-flow-unread`
  (`drops/ref_cells.rs`).

## Amendment: a library's cell only its own code and constants reach is its flow's (#456)

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the user's decision
(2026-11-02). Leaving out a check that can't fail changes no run.

**In a library, only a cell its consumer may reach is counted by its
type; one only the library's own code reaches is as in a program.**
#445's rule counted by its type each cell of a class `TOP` reaches. `TOP`
was also every constant's value but a thread-local key's. So the corpus's
`untagged_ref_cells`, whose `total(&Shared::One(4.0))` gives a promoted
constant where `total(&shared)` gives its own cell, printed
`$borrow(shared).reduce(..)` and `$borrowMut(shared).push(3)` as a
library, where its program printed `shared.reduce(..)` and
`shared.push(3)`. Each of its items is private: no consumer can reach
`shared`.

What a consumer reaches of a library is what its API takes and gives:
an exported function's parameters and result, a trait's method, which
its generic code may call, a coroutine, a `pub` thread-local, a static,
and every field of what it holds. Each is `TOP` already
(`library::reachable`, rustc's own answer, for the functions). So is what
JS, a binding, another library, a `dyn` or a fn pointer is given and
gives: JS may hand a consumer what the library gave it. A constant isn't
any of these:

- **A constant's value holds no cell another code has.** Rust refuses a
  constant that refers to memory a cell may change, and promotes no
  value with a cell in it. A constant's value is a new one where it's
  read, or memory no cell is in.
- **A static is the exception**: its memory is the one each reader
  names, the library's and its consumer's. `check_static_ptr`'s.

So a constant's value is `CONSTANT`, a node of its own, and a static's
`TOP` (`analysis/cell_flow.rs`). A cell beneath `CONSTANT` is still one
whose flow isn't followed, `Anything`, as before, so a program's classes
and JS are what they were. Only `shared`, whether `TOP` reaches it,
tells a library which of those its consumer may reach
(`CellClass::Anything { shared }`):

- **One `TOP` reaches** is counted by its type, as #445 said.
- **One only `CONSTANT` reaches** is quiet where no borrow of a type that
  may be it of such a cell is counted, as in a program
  (`RefCells::quiet_class`).
- **Its contract's `unchecked`** lists only types of cells `TOP` reaches:
  a consumer's borrow is never of another (`RefCells::borrowed`). A
  library's manifest may list fewer types than before, so its consumers
  are rebuilt against it; its form is the same.

Built as a library, `untagged_ref_cells` is now its program's JS.

### Alternatives

- **A `pub` struct's private field as no consumer's.** Rust lets no
  consumer name one, so the decision's list of what a consumer reaches
  names only `pub` fields. But JS reaches each property of a value it
  holds, as a library's `Tally.cell(tally)`, a private method, is a
  property of its exported `Tally`. A struct JS may make holds what JS
  made, and a `#[repr(transparent)]` struct's value, an untagged enum's
  and a flattened field's are their field's. A flow that left out
  private fields would have to know each of those, which the
  representation owns. Not done: such a cell is checked by its type, as
  before, and the test says so.
- **Constants as cells of their own, `Own`**: sound, as a constant's
  value holds no cell another code has, and would leave out more checks
  in programs too. That's a change of the program's classes, not of what
  a library shares: not this issue's.
- **Each constant a fresh node**: the same, and more nodes.

### What holds it

- `test/crates.test.ts`, "a library's cell only its own code and
  constants reach is its flow's, as a program's", pair
  `ref_cells_flow_reach`, against native Rust: a cell an exported
  function takes or gives, a `pub` static's, a `pub` field's and a `pub`
  struct's private one are checked; `Stock`'s and `Label`'s, which only
  `stocked` and `labelled` and a constant reach, are plain; and the
  contract lists no type borrowed unchecked, so its consumer, holding a
  borrow of a `RefCell<Vec<String>>` of its own while the library's code
  runs, isn't refused.
- The corpus's `untagged_ref_cells`, built as a library, as the nightly
  builds every case, is its program's JS.
- `test/crates.test.ts`' "a library checks a cell its consumer may share
  as its type says" and "a library's RefCells are the values they hold",
  and `test/lowering-cell-flow.test.ts`, as before.
- Mutations: `flow-constant-unseen`, `flow-static-a-constant` and
  `flow-shared-unread` (`analysis/cell_flow.rs`);
  `class-unshared-by-type-in-a-library` and `library-borrowed-unshared`
  (`drops/ref_cells.rs`).

## Amendment: a guard read only for a binding's argument (rd427)

Case ([0262](0262-when-rust-and-js-disagree.md)): B, the user's decision
(2026-11-02). React's Flight client gives a module's loader the
response's nonce, and its DEV `then` stamps the promise it makes with the
chunk's debug info:

```js
prepareDestinationForModule(response._moduleLoading, response._nonce, metadata);
wrapperPromise._debugInfo = this._debugInfo;
```

A port writes `prepare_destination_for_module(response._module_loading
.get(), response._nonce.borrow().as_deref(), metadata)` and
`js::set(promise, "_debugInfo", chunk._debug_info.borrow().clone())`.
Rust holds each guard until its statement ends, while the binding runs,
which may call back: so the first was refused where `shared`, a library,
borrows a cell of its type unchecked, and the second counted, `$borrow`
and `$unborrow` around the write; the port bound a `let` first, `const
nonce = response._nonce;`.

**A guard read only for a binding's argument, what it guards lent
(`&*guard`), a copy of it (`*guard`), or a clone or an `Option`'s
`as_deref()` of it lent, given straight to a binding's call, `js::set`
too, is held while that argument and those after it run, not while the
binding does** (`drops/facts.rs`'s `given_of_guard`): its borrow is
momentary, so neither counted nor refused, and the JS is upstream's. A
later argument's code, which runs while it's held, is asked as before:
`log_time(*t.borrow(), renamed(r))`, where `renamed` writes `t`, still
counts it and panics as Rust's does. A guard given to the crate's own
function, a library's, a `dyn`'s or a closure's is held through the call,
as before.

Only a binding that calls back at once and borrows that cell mutably
tells them apart: Rust panics `already borrowed`, and the JS passes.
Listed in ADR 0262's Consequences and the guide's differences
(`semantics.md`, 44).

Held by `test/lowering-cell-borrows.test.ts`'s "a guard read only for a
binding's argument is held while it's read, not while the binding runs",
and the corpus's `argument_guard_reentered`, against native Rust; mutations
`argument-guard-held-through-binding`, `argument-guard-of-any-call`,
`argument-guard-of-a-clone-only` and `argument-guard-later-arguments-unasked`
(`drops/facts.ts`), and `as-deref-unrecognized` (`recognition/wrappers.ts`,
whose `is_as_deref` says which call is an `as_deref`).


## Amendment: a `RefCell` that's a box is declared `{ value: T }` (#503)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the JS is as it was, and its `.d.ts` now says what it is.

A `RefCell` that isn't the value it holds, nor the object it is, is a
`{ value }` box: `fn counted(cell: &RefCell<u32>)` reads `cell.value`.
Its declaration was `any`, `export function counted(cell: any): number;`,
looser than the box, where a lent `Cell`'s is `{ value: number }` (ADR
0288). Found in the alien-signals port, an effect's `cleanup:
RefCell<Option<Rc<dyn Fn()>>>` in an enum variant's field, declared
`cleanup: any`.

**A `RefCell` that's a box is declared `{ value: T }`, as a `Cell` that's
one is**: `declarations.rs`'s `ts` asks one question of both, after a
`RefCell` that's what it holds (`is_value_cell`) and before anything
else. Its non-enumerable `borrows`, where it's counted, isn't declared,
as no JS reads it.

Held by `test/declarations.test.ts`'s "declarations type what a module
exports", `counted(cell: { value: number })`, and mutation
`boxed-refcell-declared-any` (`declarations.ts`).

## Amendment: an enum variant's `RefCell` field holds its value by its declaration (#503)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference in what the program does: each reader and writer of the field
reaches the one property, as a struct's does; what changes is the shape JS
sees, which its declaration now says.

alien-signals' effect node, `NodeKind::Effect { r#fn, cleanup:
RefCell<Option<Rc<dyn Fn()>>> }`, flattened into its node (ADR 1071), was
made `cleanup: {}` and set `e.cleanup.value = e.fn()`, a `{ value }` box,
where upstream's `ReactiveNode` has `cleanup` the function or `undefined`.
A struct's field held its value by its declaration (above); a variant's
was a box, as every `RefCell` of its type that isn't the object it holds:
`ref_cells.rs`'s `struct_field`, which each place that asks whether a
field holds its value asks, said `DefKind::Struct` alone.

**A variant's `RefCell` field holds its value by its declaration, as a
struct's does**, a tagged enum's `{ TAG: "Effect", cleanup }` and a keyed
one's own key, of named fields and of unnamed, `_0`: `ref_cells.rs`'s
`object_field`, which `struct_field` is now, says a variant's field is a
property of the object its value is, and every place that asks a field
whether it holds its value asks it so. A variant's field is reached through
MIR's `Downcast`, which `mir_field` and `place_field` already read of the
variant they're read as, and `adt_value` makes a variant's fields as a
struct's: `representation.rs`'s `struct_field_def`, of a struct's field
by its index, is asked only where the value is a struct's.

- **Made**, its `None` is the key kept, `cleanup: undefined`, as a
  struct's cell field's is (ADR 0280, amended).
- **A pattern's binding of it** lent it, as a struct's did (above): its
  one cell, `$fieldCell(e, "cleanup")`, whose `value` is the property; one
  by reference is the field's own place now (below).
  Moved out whole, `let Outcome::Finished { lines, tries } = outcome;`, a
  cell of its own of what it held.
- **Its enum's derives** read it as a struct's do: `Clone` copies a
  variant holding one, `{ ...a }`, of what it holds a clone, and `==` and
  `Debug` read what it holds (`std_impls.rs`'s `clone_fields`, given the
  variant). A variant that holds a cell that's the property is copied
  however the enum is used, as its cell is set in place; before, a
  `RefCell` field's own `needs_clone` stood for that.
- **A constant of the variant**, a thread-local's `const { Kind::Pair(
  RefCell::new(5), 1) }`, is its initializer's, as a struct's of a
  `RefCell` field is (`representation/consts.rs`).
- **Declared** as what it holds, `cleanup: (() => void) | null |
  undefined`.

An untagged enum's payload is the value it holds (ADR 0214), so its
`RefCell` is one only of an object, the object (`untagged::held_payloads`
refuses any other); std's generic enums' fields are declared `T`, no
`RefCell`. Neither needs a case of its own.

Held by `test/lowering-field-cells.test.ts`, "an enum variant's RefCell
field holds its value by its declaration", given objects JS made, the
issue's program and a keyed enum's; and the corpus's
`refcell_variant_fields` (alien-signals' node and a tagged enum's, made,
borrowed and replaced through bindings, its variant replaced whole, and
one moved out), `refcell_variant_derives` (`Clone`, `==` of a hand-written
`==`, `Debug`, `{:#?}`, a thread-local of a constant),
`refcell_generic_variant_field` and those that must panic as std's does:
`refcell_variant_field_held` (a binding given on, held while another
borrows it), `refcell_variant_field_twice` (`already borrowed` through one
binding), `refcell_variant_replaced_held` (the enum's own cell replaced
while a variant's field is borrowed), `refcell_variant_derive_meets` (a
derived `Clone` that borrows it again) and `refcell_generic_variant_held`,
each against native Rust. Mutations `variant-refcell-boxed` (`drops/ref_cells.ts`),
`variant-clone-boxes-cells`, `variant-clone-of-cells-shared` and
`variant-eq-boxes-cells` (`std_impls.ts`), `variant-refcell-const-boxed`
(`representation/consts.ts`) and `variant-refcell-declared-box`
(`declarations.ts`).

## Amendment: a pattern's binding by reference of a field is that field's own place (#503)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the binding's borrows are the field's, as `x.f.borrow()`'s
are, and each that may meet another is counted on the field's one cell,
which panics where std's does.

A variant's field is reached only through a pattern, `let
NodeKind::Effect { r#fn, cleanup } = &e.kind else { .. }`, and a pattern
that binds a field lent it (above): so each variant's `RefCell` field was
its one cell, `$fieldCell(e, "cleanup")`, where alien-signals writes
`e.cleanup = e.fn()`. The binding is a `&RefCell` to the field, as `&x.f`
is, but no more is done with it than with `x.f` of `x.f.borrow_mut()`:
borrowck keeps it the field for as long as it's bound.

**A pattern's binding by reference of a struct's or a variant's `RefCell`
field, `cleanup` of `&e.kind`, `ref cell` too, is that field's own place**
(`ref_cells.rs`'s `bound_ref_cells`): each use of it is the field's. One
that's a borrow of it, `cleanup.borrow_mut()`, `replace`, `take` or
`get_mut`, is a borrow of the field, counted where it may meet another
(`counted_in`); any other, given on, `bump(cleanup)`, or captured, lends
the field, as `&x.f` does. So `quiet_field` follows it: a field none of
whose borrows can meet another is the property, `e.cleanup = e.fn()`.

- **A binding by value**, moved out, `let Outcome::Finished { lines, tries
  } = outcome;`, or `mut`, is a value of its own, as before: the field
  lent, `{ value }` of what it held.
- **A variant's field is quiet** where its bindings' uses are, as a
  struct's: the quiet fields are each struct's and each enum's. A
  derive's read of one, `Clone`'s, `==`'s, `Debug`'s, is asked as a
  struct's is (above).
- **MIR's**, `_5 = &((*_1) as Effect).1` and `borrow_mut(copy _5)`: the
  field its `Downcast` reads (`local_facts.rs`'s `lent_field`), so its
  guard releases nothing. A binding of a variant's `RefCell` field that's
  the property is a handle on that place, read where it's used
  (`handle_on_kept_field`), of a variable nothing sets again, `kind` of
  `kind.cleanup`, as borrowck keeps what it's in: what the pattern matched
  is fixed where it's matched, `const job = task.job;`, so a variable the
  program sets after, `task = second`, changes no binding of it. Each of its
  borrows is then the place, `e.cleanup = e.fn()`, where a `const` of its
  handle read `{ get value() { return e.cleanup; }, .. }` (#542). A
  `Cell`'s binding is a `const` of its handle as before, which
  `read_handles` reads as ADR 0287 (amended, #532) says: once, where it's
  read; twice, held once. And a guard's handle a `let` declares only once
  it sinks to its branch is read as its place too (`tidy/values.rs`'s
  `read_handles`).
- **A generic variant's `RefCell<T>` field made where its variant is**,
  `Slot::Full { current: RefCell::new(value) }`, and a binding of it, are
  the field's own, no `RefCell<T>` of any value (`made_places`,
  `property_reads`), as a struct's are; and where some borrow of it may
  meet another, each `RefCell` of an object it's made of is a box, as a
  generic struct's is (`of_objects`), so its generic code's `$fieldCell`
  and its concrete code's agree.

```rust
use std::cell::RefCell;
use std::rc::Rc;

pub enum Kind {
    Effect { cleanup: RefCell<Option<Rc<dyn Fn()>>> },
    Scope,
}

pub fn set_cleanup(kind: &Kind, cleanup: Rc<dyn Fn()>) {
    if let Kind::Effect { cleanup: cell } = kind {
        *cell.borrow_mut() = Some(cleanup);
    }
}
```

```js
export function setCleanup(kind, cleanup) {
  if (kind.TAG === "Effect") {
    kind.cleanup = cleanup;
  }
}
```

A struct's pattern binding by reference, `let Wrap { n } = &wrap;`, is so
too: the one rule of a field's binding. Its uses before, a call given it,
lent it, and still do: corpus `refcell_bound_field_held` is as it was.

The readability baseline's corpus gains the cases below; one existing
output reads a handle's place where it was a `const`, the late
`read_handles`' `match_arm_guard_first`'s `$index(pair, 0).sibling =
node;`.

Held by `test/lowering-field-cells.test.ts`, "an enum variant's RefCell
field holds its value by its declaration", now its binding the field, no
`$fieldCell`, no `.value`; the corpus's cases of the amendment above,
their JS the property's where nothing meets, against native Rust, and
`refcell_variant_fields`'s `moved_on`, a binding of a task whose variable
is set to another after. Mutations `variant-refcell-never-quiet`,
`bound-place-lent`, `bound-place-given-on-unlent`,
`bound-place-borrow-uncounted`, `variant-derive-unasked`,
`variant-made-a-value`, `bound-place-read-any-value` and
`generic-variant-objects-unboxed` (`drops/ref_cells.ts`),
`variant-field-reference-held` (`mir/assign.ts`) and
`late-field-handles-unread` (`mir/tidy/values.ts`); and
`plain-refcell-bound-kept`, of a field moved out, follows the binding by
value. Two hold no mutation: that a handle's root is a variable nothing
sets again, as what a pattern matched is fixed where it's matched, so no
binding's root is one set again; and `lent_field`'s variant, as each
corpus case's flow tells its cell apart already, its borrow quiet by its
class (`quiet_cell_borrow`), where a struct's `x.f` is asked the same.

## Amendment: #503's two points confirmed (the user's decision)

Decided by the user (2026-11-04), of the two amendments above
(80814f061..c85a56a19):

- **The rule's extension to a struct's `ref` bindings stays**: a
  struct's pattern binding by reference of a `RefCell` field, `let Wrap {
  n } = &wrap;`, is the field's own place, as a variant's is; one rule of
  a field's binding.
- **A `RefCell` field's binding read twice with no write between keeps
  reading the field twice**: its handle is read where it's used, each
  read the property's, and isn't held in a `const` as a `Cell`'s is (ADR
  0287, amended, #532).

## Amendment: a `take`'s `Default` runs before its borrow (#699)

"Where std's code holds it while the crate's runs: .. a `Default` a
`take` runs" was false: std's `take` makes its `Default` before it
borrows, so a `take`'s borrow is held for a moment, as a `replace`'s is,
and a `Default` of the crate's is made first, before the borrow is
checked. It's [ADR 1044's amendment](1044-a-flattened-state-is-its-classs-own-keys.md#amended-699-its-cells-own-take-compiles-its-default-made-first),
where a flattened state met it.
