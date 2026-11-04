# 1088. Whether a tree sets a variable is `js::Writes`' answer alone

Status: Accepted (#523, the user's order of 2026-11-04 through the
coordinator: one write-query owner). Extends
[0670](0670-a-trees-writes-have-one-owner.md), whose `js::Writes` said
which variables a tree writes, to the one question every pass asked of
its own: does this tree set `x`, and how often.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A pass asks what it
asked, of a fuller answer; the corpus's JS is the same byte for byte.

## Context

ADR 0670 gave a tree's writes one owner, `js::Writes`. Two passes kept
their own answer to "does this tree set `name`", each a different one:

- `tidy/bindings.rs::assigns`, asked by 19 call sites in tidy (loops'
  bounds, moves, destructuring, decided tests, shadows, `for…of`'s
  bindings): an `x = ..` statement in the tree's own blocks. Not `x +=
  ..`, not `(x = ..)` in an expression, not a closure's, not a handle.
- `js/applied.rs::sets`, of applied-arrow inlining: `x = ..` and `x +=
  ..`, closures too. Not `(x = ..)`, not a handle.

So did two more: `mir/scopes.rs`' "a parameter nothing assigns", and
`classes.rs`' "a constructor whose instance is set again". Each missed
writes another counted, and a pass that asks "is `n` set?" to move a read
of `n`, or to keep a loop's bound, is wrong where the answer misses one:
the family of wrong answers the readability study names F-READ (#452,
#485, #515).

## Decision

**`js::Writes::sets(stmts, name)` says whether a tree sets a variable's
binding, and `Writes::count(stmts, name)` how many places do; every pass
asks them, and none matches a write of its own.**

- A set is `x = ..`, `x op= ..`, `(x = ..)` in an expression, `x++`
  (`ExprKind::Stepped`, ADR 0948), and a handle on `x` (`&mut x`, ADR
  0099), which what it's given may set; in any block, a closure's and a
  declared function's too, each counted once wherever it is.
- `Writes` sees `Stepped` as it sees `Assigned`, in its `replaced` and
  `replaced_in_closures`, so `may_replace` and `by_closures` answer it
  too.
- `assigns` and `applied::sets` are gone; their callers, and scopes' and
  classes', ask `Writes::sets`. Each asked to stay safe, so a fuller
  answer only keeps more as it was: no output of the corpus, the lowering
  tests or the guide changed.
- **Held by an architecture test**: no `src/` file but `js/writes.rs`
  searches a tree's statements for an `Assign` of a variable by name
  (`.any(`, `.filter(`, `.position(`, `.find(` or `|=` of such a
  `matches!`). A pass that matches one statement's shape to rewrite it,
  `x = y; return x;`, isn't asking whether the tree sets it; the few that
  look for such a shape across a list are listed by name in the test, each
  saying what shape it matches.

## Consequences

- A pass that needs a variable set nowhere, or once, asks `Writes`; a new
  kind of write is added there, and every pass sees it.
- Mutations: `writes-closure-set-uncounted` holds a closure's set, by
  the corpus case `for_binding_set_by_its_closure.rs` (#663). Two are
  gone as equivalent now: `tuple-part-set-by-closure`, which left out
  `tuple_reads`' set of what closures and handles anywhere in the body
  set, since `Writes::sets` of what's after the tuple counts a closure's
  set, and Rust's borrows forbid reading the part while one made before
  it is live; and a mutation of `(x = ..)` counted, which no pass that
  asks `sets` meets in the corpus (`x++` none at all: it's made after
  every pass that asks). The unit tests of `js/writes.rs` hold both.
