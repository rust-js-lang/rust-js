# 0868. What std's impls and derives run is what their bounds say

Status: Accepted. Amended (2026-11-03): of a type of a type parameter too
(#499). Amends [0863](0863-a-refcell-of-an-object-is-the-object.md)
and [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md): what
std's code a call runs may borrow, of the types it's given.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A borrow is left
unchecked only where no other can meet it: std's impl of a trait calls
what it's given only through its bounds, as std's functions do (ADR
0328), so a type its bounds don't name runs nothing there. Where they
do, a derived `Clone` whose field's `Clone` borrows the cell, the borrow
is counted, and panics as Rust's does.

## Context

React's `act` takes the queue its scope pushes to, and its renderers'
tasks, as an `Rc` it shares:

```rust
#[derive(Clone)]
#[repr(transparent)]
pub struct RendererTask(pub &'static dyn Fn(bool) -> Option<RendererTask>);

pub struct SharedStateClient {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub act_queue: RefCell<Option<Rc<RefCell<Vec<RendererTask>>>>>,
    ..
}

let prev_act_queue = r.act_queue.borrow().clone();
..
let mut callback = queue.borrow()[i].clone();
```

The `RefCell` is borrowed while std's `Clone` of what it holds runs. To
say what that may borrow, ADR 0863 walked every type `Option<Rc<RefCell<
Vec<RendererTask>>>>` is made of, as though each impl ran each part's:
so `RendererTask`'s derived `Clone`, and its field, a `dyn Fn`, which may
be anything. Every borrow of `act_queue` was "held while code that may
borrow it runs", and the nullable field was refused (ADR 0772). Yet `Rc`'s
`Clone` clones no `T`: it counts. Nor does a `&T`'s, which copies the
reference. And a call of a derived impl, `queue.borrow()[i].clone()`, was
a body of the crate's never lowered, so it, too, might run anything.

## Decision

**What std's impl of a trait runs of a type is what its bounds say**
(`cell_borrows::std_impl_bounds`): `impl<T: Clone> Clone for Option<T>`
runs `T`'s `Clone`, `impl<T: ?Sized> Clone for Rc<T>` none, `impl<T:
?Sized> Clone for &T` none. Each trait of the bound's, and each it
extends with methods of its own, is selected as rustc selects it, and
what that impl is bounded by is walked in turn. Where one isn't std's
impl, is built in, is of a trait of more arguments (`Fn(&T)`,
`PartialEq<Rhs>`) or a trait alias, or the type has a parameter, the
walk is ADR 0863's, of every part. **A function pointer's impls never
call it**, but an `Fn` trait's.

**A method std derives, `#[derive(Clone)]`'s `clone`, runs what it reads
of its type's fields by its trait**, as std's code bounded by that trait
does: a call of it asks what the derive's walk of the fields says, not
"anything" of a body never lowered.

So neither React cell is counted: `act_queue` is the nullable field, a
plain property, and the queue the array, read unchecked:

```js
const prevActQueue = ReactSharedInternals.actQueue;
const queue = prevActQueue ?? [];
ReactSharedInternals.actQueue = queue;
..
while (i < queue.length) {
  let callback = $index(queue, i);
```

## Why

- **It's what std's code does**: its impls are generic over their
  parameters, and call them only through their bounds, which rustc
  checks.
- **It's React's**: the array and the field its renderers read.

## Alternatives

- **Walk every part, as before**: a `dyn` deep in a type an `Rc` holds
  makes every borrow of a cell of it counted.
- **Special-case `Rc` and `&T`**: a list beside the trait system, which
  says it of every impl already.

## What holds it

- Corpus `refcell_clone_through_rc`: a nullable field's `Option<Rc<
  RefCell<Vec<Task>>>>` cloned while it's borrowed, a derived `Clone` of
  a transparent struct of a function, both unchecked; and a derived
  `Clone` whose field's `Clone` borrows a cell, which panics `already
  borrowed` as native Rust does. `refcell_eq_through_guard`'s guard,
  compared by a derived `PartialEq` of a `u32` and a `Vec<String>`, is
  unchecked.
- Mutations in `scripts/mutations/lower/analysis/cell_borrows.ts`:
  `std-impls-walk-every-part`, `std-impl-bounds-skipped`,
  `derived-methods-run-anything`, `derive-walk-skipped`,
  `fn-pointer-clone-runs-it`.

## Amendment: std's function given to std's runs what a call of it runs

Case: A, as above.

React's ReactFiberWorkLoop keeps `pendingViewTransitionEvents`, a
`null` or an array of callbacks, and pushes each, `if
(pendingViewTransitionEvents === null) pendingViewTransitionEvents = [];
pendingViewTransitionEvents.push(event)`. Its port, `EVENTS.with_borrow_mut(
|events| events.get_or_insert_with(Vec::new).push(event))` of a
`RefCell<Option<Vec<&dyn Fn(&[String])>>>`, made the thread-local a
checked cell, `$withBorrowMut(events, ..)`, where the same of a
`Vec<u32>` was a `let` (the port's gap wl2). `get_or_insert_with`'s
`F: FnOnce() -> T` is `Vec::<&dyn Fn(..)>::new`, a function item of std's,
and `bound_runs` walked each type it's of, the `dyn Fn` among them, as
what the bound may call: anything.

**A function item of std's given by a bound is called as std's code is:
it runs what a call of it runs, `std_given`'s of its own bounds**, the
one rule a direct call of it has (`called`'s `Called::Std`). `Vec::new`
has none with methods, and runs nothing; a cell's own `Clone::clone`,
`.map(RefCell::clone)`, still borrows it. A function of the crate's,
given so, is still its body, which may borrow the cell held:
`get_or_insert_with(first_hooks)` of one that reads the thread-local
panics `RefCell already mutably borrowed`, as Rust's does.

Held by `test/lowering-cell-borrows.test.ts`'s "std's function given to
std's runs what its bounds say, not what it's of", both; mutation
`std-function-given-its-types`.


## Amendment (2026-11-03): of a type of a type parameter too (#499)

Case: A, as above.

alien-signals' graph is generic over its node's kind, `Link<K> { dep:
Rc<ReactiveNode<K>>, next_dep: RefCell<Option<Rc<Link<K>>>>, .. }`, and
reads each link for a moment, `let next = link.next_dep.borrow().clone();`.
Once its crate had a closure JS calls with rest arguments, whose code
rust-js can't see, every one of those borrows was counted, and each field
was its one cell: system.js read `const borrow = $borrow($fieldCell(link,
"nextDep"), true); const next = borrow.value; $unborrow(borrow);` where
upstream reads `link.nextDep`. `std_impl_bounds` gave up on any type with
a type parameter in it, so `Option<Rc<Link<K>>>`'s `Clone` was taken to
run anything, although it clones only an `Rc`, which clones no `K`. The
cost was more than the reading: each borrow defined a property on its
cell, and the port ran 55 to 280 times slower than upstream (149 ms per
update of a 100 by 100 grid, against upstream's 0.5 ms).

**Std's impl of a type of a type parameter is selected as one of no type
parameter is, and runs what its bounds say**: `Option<T>`'s `Clone` its
`T: Clone`'s, `Rc<T, A>`'s its `A: Clone`'s, `Global`'s nothing. A type
parameter itself, `K: Clone`, selects no impl of std's, and stays what any
type parameter is: anything, where the crate has impls of its own. So
`slot.item.borrow().clone()` of a `RefCell<Option<K>>` is still counted,
and panics `already borrowed` where `K`'s clone borrows the cell, as
Rust's does.

The port's system.js and index.js have no borrow helper left, and upstream's
suite passes. A 100 by 100 grid's update takes 1.3 to 1.6 times upstream's
(0.65 ms against 0.42 to 0.51 ms), and making 100,000 signals, computeds
and effects 2 times (88 ms against 44 ms), on a loaded machine.

Held by `test/lowering-cell-borrows.test.ts`'s "std's impl of a type of a
type parameter runs what its bounds say", the graph's plain read and the
slot's counted borrow, run; mutation
`std-impl-bounds-of-a-param-type-unread` (`lower/analysis/cell_borrows.ts`).
