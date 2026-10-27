# 0960. A cell's `dyn` is any type only where a value is made one in place

Status: Accepted. Amends [0863](0863-a-refcell-of-an-object-is-the-object.md),
which cells may be one another (`may_be_cell`), and [0397](0397-a-refcell-of-a-dyn.md)'s
`RefCell<dyn T>` that shares its cell. Fixes #374. Amended (2026-10-27): a thread-local's own
cell only its key reaches is no other cell of its type.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Which borrows are
checked is rust-js's own judgment of which can meet; a cell no borrow of
which can meet another's behaves the same checked or not.

## Context

A borrow of a `RefCell` is checked, `$borrowMut(cell, true)`, where one
of a cell that may be it is held while code that may borrow it runs (ADR
0863). `may_be_cell` took a `dyn` anywhere in a cell's type for any type,
so React's test selectors' `COMMIT_HOOKS: RefCell<Vec<Rc<dyn Fn()>>>`,
borrowed while its hooks run (which may run anything a JS caller gave),
"may be" the hydration diffs' `RefCell<Vec<Rc<HydrationDiffNode>>>`: adding
the test selectors checked every borrow of a node's `children` and the
hydration errors, in two modules that never meet the hooks. So did a
strict-mode warnings cell for CacheComponent's listeners (#374).

A cell is seen with another type only where a value is made unsized in
place: `Rc<RefCell<Circle>>` made `Rc<RefCell<dyn Shape>>`, the cell's own
value, or a struct's last field, `RefCell<Framed<dyn Shape>>`, as Rust's
`Unsize` of a struct is. A `Vec`'s items and what an `Rc` points at are
behind a pointer, and no `Vec<Rc<Node>>` is ever a `Vec<Rc<dyn Fn()>>`.

## Decision

**In `may_be_cell`, a `dyn` is any type only at a position a value is made
unsized in place**: the top of the cell's type, and a struct's argument
its last field is the tail of (`unsized_in_place`, through `RefCell<T>`'s
`UnsafeCell<T>` to `T`). Elsewhere it's a `dyn` of its trait, which `head`
compares as any other constructor, and a type parameter is still any
type. A library's contract shapes compare so too (`may_be_cell_shape`).

## Why

- **It's which cells can be one**: two cells meet only if one can be made
  the other, and a `dyn` deeper than a value's tail can't.
- **React's modules keep their plain arrays**: the hydration diffs'
  `siblings.push(newNode)`, as upstream writes it, beside the test
  selectors' hooks, whose own cell stays checked, as Rust panics if a hook
  observes again while they run.

## What holds it

- `test/lowering-cell-borrows.test.ts`, "a borrow held while hooks run
  checks only cells that may be its own", run; `test/crates.test.ts`, "a
  library's checked cell of a Vec of dyn Fns is no consumer's cell of a Vec
  of its own"; `test/corpus/refcell_dyn_alias_borrow.rs`, the cell's own
  value, against native Rust.
- Mutations: `cell-dyn-anywhere`, `cell-unsized-tail-unseen`,
  `cell-may-be-ignores-dyn` (`scripts/mutations/lower/analysis/cell_borrows.ts`),
  `cell-shape-dyn-anywhere` (`scripts/mutations/lower/library.ts`).
- A struct's last field made `dyn` inside a cell, `RefCell<Framed<dyn
  Shape>>`, isn't tested against native Rust: its coercion is a wrong
  answer of its own today, #375.

## Amendment: a thread-local's own cell only its key reaches is no other cell (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above: which
borrows are checked is rust-js's judgment of which can meet.

React's test selectors call their commit hooks while `COMMIT_HOOKS`, a
`RefCell<Vec<Rc<dyn Fn()>>>`, is borrowed, which Rust must check, as a
hook may observe again. That counted every cell of the type, so
ReactFiberCacheComponent's AbortControllerShim, whose `listeners` are an
`Rc<RefCell<Vec<Rc<dyn Fn()>>>>`, checked each borrow of them,
`$borrowMut(listeners, true)` where upstream writes `listeners.push(listener)`
(ts6 of the port's gaps). No hook can borrow `listeners` while the hooks'
borrow is held as a borrow of `COMMIT_HOOKS`: they're two cells.

**A borrow of a thread-local's own cell by its key, of a key only the
crate's code reaches and never lends (`closed_keys`, ADR 0580, amended:
private, each use `with_borrow`, `with_borrow_mut`, `get`, `set`, `take`
or `replace`, never `with`), counts that cell alone** (`RefCells::
key_counted`, asked by `CellBorrows::reached_only_by_key`): no `&RefCell`
anywhere is it, so a borrow of a cell through a reference, a field, an
`Rc`, is never of it, and its type isn't counted for them. The key's own
writes, `HOOKS.set(..)`, `take`, `replace`, which `quiet_ref_cell` asks of
its type, still count it (`of_key`), as its `with_borrow`s are checked:
a hook that sets the hooks while they run panics, as Rust's does. A key
its `with` lends, or a public one, which another crate's code may lend,
counts every cell of its type, as before.

- What changes: React's ReactFiberCacheComponent's listeners are plain
  arrays beside ReactTestSelectors, as upstream; ReactStrictModeWarnings'
  `$assign($borrowMut(pendingComponentWillMountWarnings), [])`, a key's
  own write while its borrow may be held, stays checked. No corpus case,
  snapshot or guide example changed.
- Held by `test/lowering-cell-borrows.test.ts`, "a thread-local's borrow
  held while hooks run checks no other cell of its type", run: the
  controller's listeners plain, the hooks still panicking where one adds,
  sets or takes them, and a key its `with` lends keeping every cell of its
  type checked; the mutations `key-borrow-counts-its-type`,
  `key-counted-unasked` (`drops/ref_cells.ts`),
  `lent-key-reached-only-by-key` (`analysis/cell_borrows.ts`),
  `key-set-of-any-cell` and `key-take-of-any-cell` (`std_types/cell.ts`).

## Amendment: what code rust-js can't see borrows is no thread-local only its key reaches (2026-10-27)

The amendment above made a thread-local's own cell, which only its key
reaches, no other cell of its type where a borrow is held. What code
rust-js can't see may borrow, a binding's call run under a `with_borrow`,
was asked still of every borrow the crate's later bodies make, a generic
`RefCell<T>`'s through a reference among them, `cursor.current.borrow_mut()`,
which may be any cell's. React's DEV dispatchers call
`REACT_SHARED_INTERNALS.set_h(dispatcher.as_deref())`, a library binding,
under `X.with_borrow(..)`, and each of the seven was a `{ value: null }`
cell read by `$withBorrow`.

**Of a thread-local only its key reaches, what unseen code may borrow is
what the crate's later bodies borrow of it by its key**
(`anything_asks`): a borrow through a reference is never of its cell,
which none reaches. React's dispatchers are their module's variables,
`ReactSharedInternals.H = InvalidNestedHooksDispatcherOnMountInDEV ??
null`. Held by `test/lowering-cell-borrows.test.ts`'s "a thread-local only
its key reaches is no cell a borrow by reference is of"; mutation
`closed-key-type-borrow-asks`.

