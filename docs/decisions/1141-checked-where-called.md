# 1141. A momentary borrow and `Vec::remove` are checked where they're called, then JS's own

Status: Accepted (2026-11-14, the lead's decision, after #1692); amended (the check first only where the borrow is evaluated first).
Amends [0328](0328-refcell-borrow-counts.md)'s form of a momentary borrow and
[0576](0576-what-a-bodys-integers-can-be.md)'s `$removeAt`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, none; what each does
is as before, panics and their messages included. Only where the check
is, and that the path that runs makes no call, change.

## Context

React's Fizz recovers from a tree too deep for the stack inside
`js::catch_stack_overflow` (ADR 1140), and its replayElement's `catch`,
at the deepest frame, consumes its slot, `replay_nodes.borrow_mut()
.remove(i)`. Its JS was `$removeAt($borrowMut(replayNodes), i)`: two
runtime calls where upstream's `replayNodes.splice(i, 1)` makes one, and
the deepest frame had room for neither, so "can recover from very deep
trees during resume" failed wherever the stack was laid out tighter. The
guide's rule says helpers are emitted only where behavior requires them;
neither call is, where nothing fails.

## Decision

- **A momentary borrow is checked where it's taken.** A guard held only
  while nothing can ask (rd218), whose whole use runs no code of the
  program's, a `Drop`, a closure or a dictionary's call, was
  `$borrowMut(cell)`, checked, not counted. It's `if (cell.borrows)
  $alreadyBorrowed();`, a shared one's `if (cell.borrows < 0)
  $alreadyMutablyBorrowed();`, then the cell: no flag set and cleared, as
  before, and no call where the cell is free. Which guards are momentary
  is the borrow owner's, as it was (`drops::facts`); a guard held while
  the program's code runs is counted, `$borrow(cell, true)` and its
  `$unborrow`, as before. The two helpers throw what `$borrowMut` and
  `$borrow` throw, one message each.
- **`Vec::remove(i)` is checked where it's called:** `if (i >=
  items.length) $removeOutOfBounds(items, i);`, then JS's own
  `items.splice(i, 1)`, its item `[0]` where something takes it. Its
  panic is std's, `removal index (is 3) should be < len (is 1)`, thrown
  from the runtime as `$removeAt` threw it. An `items` or an `i` that
  isn't read the same twice is a `const` first.
- **A value nothing reads keeps what it does** (`Expr::read_alone`): an
  item read whose index does nothing is left out, `nodes.splice(i,
  1)[0];`'s `nodes.splice(i, 1);`. A property's read, which a getter may
  answer, and one a getter answers, are kept.

React's replayElement's line is now three: the two checks, and
`replayNodes.splice(i, 1);`, upstream's.

## Consequences

- `test/borrow-forms.test.ts`: the forms of `consume`, the item taken
  where something takes it, out of bounds and already borrowed each
  native Rust's panic, and a guard held while a `Drop` runs counted as
  before. Mutations call a momentary borrow again, refuse a shared one
  beside a shared one (`address_of_a_nested_field_through_a_deref.rs`),
  leave the bounds unchecked, keep an unread item read, and make every
  guard momentary (`momentary-always`, now of this file too).
- The corpus: 110 snapshots, each a momentary borrow or a `remove` in
  its new form, none of what they print changed. The readability
  measure finds no shape's share grown: renamed names, `x$1`, 478.8 to
  477.5 in 10,000 lines, and role fallbacks 267.7 to 266.2. A cell or an
  `items` not read the same twice, a field's `$fieldCell(..)`, is held in
  a `const` first, named by its role where its field's name is taken,
  `cell$1` beside the program's own `count`.
- The guide's examples show the forms, and `collections.md` says so.

## Amendment: the check first only where the borrow is evaluated first

Written where it's taken, in lowering, a momentary borrow's check was a
statement before the statement that reads it, and so before a loop's
test, which tidy then couldn't fold into its header: React's
`for (let j = 0; j < $borrow(segments).length; j++)` became `let j = 0;
while (true) { if (segments.borrows < 0) ..; if (j >= segments.length)
break; .. }`, six such loops in the port, where upstream writes the
`for`; a short circuit's and a conditional's too. The corpus had none,
so the readability measure didn't see it. Now:

- **The borrow owner writes the call** `$borrowMut(cell)` or
  `$borrow(cell)`, as before ([0328](0328-refcell-borrow-counts.md)),
  `Known::Momentary` (`Expr::momentary_borrow`).
- **Tidy's last pass but one writes its check first** (`borrow checks
  first`, `js::borrow_checks`) only where it's evaluated before anything
  else in its statement that does something: `if (nodes.borrows)
  $alreadyBorrowed();`, then the cell, a `const` that held the borrow then
  read as the cell where nothing after sets either. A loop's test, which
  runs again, a short circuit's or a conditional's branch, and what runs
  after a call keep the call, as before. `Vec::remove` of a momentary
  borrow writes its check first, as its bounds' (`vec_removed`).
- **The rules the first form had left unreached are kept**: ADR 0598's
  index read of `$borrow(completedSegments)[0]`, no part, and rd439's
  name for `const { value: _4 } = $borrow(cell)`, each reached again, as
  their mutations show.

Held by `test/borrow-forms.test.ts`'s "a momentary borrow in a loop's
test or a branch is the call, as before", a `while` test folded to a
`for` and a short circuit's and a conditional's branch kept, and the
mutation `momentary-hoisted-out-of-a-short-circuit`; the readability
measure's `while (true)` share, which the corpus's cases of a borrow in
a test now hold. Against the measure before this ADR, renamed `x$1`
478.8 to 477.0 and role fallbacks 267.7 to 266.0 in 10,000 lines; the
names are those before it, `const borrowMut = request.log`, of what
held the borrow.

