# 0670. A tree's writes have one owner, and its walks one definition

Status: Accepted. Extends [0659](0659-tidy-is-a-list-of-stages-with-contracts.md):
a pass works out what it reads as the tree is when it runs.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: how the compiler is
built; no program's JS changed.

## Context

A second architecture review found knowledge spread across rewrites:
which variables a tree writes was worked out five ways. Tidy's
`closures_set` (a closure's writes), `handled` (a `&mut`'s handle), its
`assigned` (both), `sets_through` (an object written through), and
preparation's `set_in` and `shadow`, and `js::constants`, each walked the
tree for its own mix. Tidy's `assigned` and preparation's `set_in` asked
one question, whether a binding may be replaced, and answered it apart:
only the latter counted `x += 1`, which preparation makes, so a reader had
to work out why they differed. And twenty analyses that only read a tree
copied it to walk it with the walk that changes one, as no walk read it.

## Decision

**`js::Writes::of(stmts)` answers what a tree writes of its variables,
each kind apart: `replaced` (`x = ..`, `x += ..`), `changed` (`x.f = ..`,
the variable at the root), each of those in a closure or a declared
function (`replaced_in_closures`, `changed_in_closures`), and `handled`
(a handle made on `x`). A rewrite puts its own condition together from
them, `may_replace(x)` or `by_closures()`, asked of the tree as it is.**

- **No universal "is safe"**: replacing a binding, changing its object,
  a closure's write and a handle are different facts, and a rewrite that
  needs one asks for it.
- **Never kept across a rewrite**, as ADR 0659 has it: `Writes` is made
  where it's read.
- **`x += 1` counts everywhere**: tidy never sees one, preparation makes
  them, and one definition serves both.

**A tree's walks are written once, `js/walk.rs`, a macro made into the
walks that read a tree, `each_list`, `each_expr`, `Expr::each`, and those
that change it, `statement_lists`, `each_expr_mut`, `Expr::each_mut`.**
They go through the same nodes in the same order. What only reads a tree
walks it as it is; the architecture test refuses a copy made to walk it.

## Consequences

- `closures_set`, `handled`, tidy's `assigned`, `sets_through`,
  preparation's `set_in`, `js::constants`' own walk and the contracts'
  `each_list` are gone, their callers `Writes`'s and the walks'.
- `rebinds` (a binding declared or shadowed), `in_closure` (a closure's
  reads) and tidy's `captured` answer other questions and stay.
- Held by `writes.rs`' unit tests (a binding replaced is not its object
  changed, a closure's writes, a handle's) and by the corpus, whose JS
  is byte for byte what it was.
