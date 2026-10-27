# 0960. A cell's `dyn` is any type only where a value is made one in place

Status: Accepted. Amends [0863](0863-a-refcell-of-an-object-is-the-object.md),
which cells may be one another (`may_be_cell`), and [0397](0397-a-refcell-of-a-dyn.md)'s
`RefCell<dyn T>` that shares its cell. Fixes #374.

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
