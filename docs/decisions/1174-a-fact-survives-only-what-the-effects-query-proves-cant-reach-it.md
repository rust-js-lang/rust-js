# 1174. A fact survives only what the effects query proves can't reach it

Status: Accepted (2026-11-19, #2112 Change B, m1-lisa's design, the lead's
approval). Step 2 of 4: the query is built and run in shadow beside the
walk's forgetting rules; step 3 makes the transfer ask it and deletes the
rules.

## Context

The MIR intervals (ADR 1137) keep each fact through a statement unless one
of about ten shape rules forgets it: `written_through_a_pointer`,
`forget_escaped`, `forget_lengths(root)`, `lent_where_borrowed`, `lent`,
the call rules `may_write_cells`, `may_write_escaped`, `called_on_mut`,
`guard_lent`, `unpush`, and the drop rules. A shape none of them lists
keeps a length, a bound or a non-zero past a write that changed it: each
of #1781, #1782, #1943, #1966, #2053, #1977, #1928, #1878, #1849 and #1885
was one, a P0. The rules fail open.

## Decision

One owner answers what a statement may write, `analysis/effects.rs`:
`Effects::of(tcx, typing_env, body, libraries, made).stale(body,
location)`, the locals nothing known of survives what's at `location`. It
fails closed: what it can't see may write.

- **Targets.** A pointer may point into a local's own storage, `Whole(l)`,
  or into what a local that holds a pointer points to, `Pointee(l)`. A fact
  of a reference's length is of the reference; a pointer assigned anew
  changes nothing it pointed to.
- **Points-to.** A flow-insensitive map of what each target's memory holds:
  a `&mut`, a raw pointer, or a shared reference to what can be changed
  through one (`changes_through_shared`: rustc's `Freeze`, refined through
  what it holds, std's types by their type arguments, the crate's by their
  fields, `&Rc<RefCell<_>>` as `&RefCell<_>`); each copy, cast and
  aggregate of one; each call's result holds what its arguments reach,
  however many pointers down, `deref_mut(&mut guard)`'s; and a call may
  store what it's given into what else it's given, of what can hold a
  pointer. A std type that holds a borrow by a lifetime, a `RefMut`, a
  slice iterator, holds a pointer, one that may write where what it holds
  is invariant; `NonNull` (`pointers::raw_pointee`) and `Box` hold one. A
  constant that holds a pointer, a static's address, points where the body
  can't see.
- **Written.** A statement writes its place's local; through a pointer,
  each target the pointer may point into, of a whole local only one rustc's
  `MaybeBorrowedLocals` says may be borrowed there. A call writes its
  destination and what each argument reaches; a drop its place, and what it
  reaches where its drop runs code. A dead local isn't written: it's read
  again only once it's assigned.
- **Escaped.** A target escapes where a raw pointer is made of it, where a
  call may keep a pointer to it (`pointers::argument_may_keep`, which asks
  `pointer_may_keep` and `closure_may_keep`: the escape owner asked, not
  re-derived), or where a pointer to it is stored where the body can't see
  or into a parameter's pointee; from there on, a forward flow. What a
  parameter that can be changed through an alias its caller keeps points
  to has escaped where the body starts, and so has a local the walk's
  rewrites made of such a place (`made` on: a parameter's field, a
  capture, what a guard lends). Code that isn't std's, a call of the
  crate's or a library's, or std's given a closure, a `dyn` or a type of
  the crate's, writes each target escaped there.
- **Stale.** A local is stale where a target it is or reaches, however many
  pointers down, is written.

The shadow (`analysis/intervals/shadowed.rs`, lines of `--intervals-shadow`)
compares, at each statement and at each call, drop, yield and assembly of
the fixed point, each fact the rules keep or forget with what the query
would: `effects-forgets` where the rules keep a fact the query would
forget, `effects-keeps` the other way. What the statement assigns is told
again by both and left out; so are a switch's, an assert's and a dead
local's, which write nothing. `scripts/effects-shadow.ts` tallies the
corpus.

## Measured

The corpus, 2,778 cases (87 refused), `bun scripts/effects-shadow.ts`:

| | lines | cases |
|---|---|---|
| `effects-forgets`, the rules keep, the query would forget | 173,743 | 1,229 |
| `effects-keeps`, the rules forget, the query would keep | 174 | 70 |

Of the forgets, by the fact's kind: a copy's relation 152,653, a `&mut`
of a `Vec` it lends 8,266, a range 3,622, `x < y` 2,366, a length 1,843,
an iterator's slice 1,663, a comparison 1,047, a slice's least 983, a held
guard 636, the rest under 250 each; an integer's interval 14. Most are of a
copy kept past a call given a shared reference to what can be changed
through one, a type parameter's or a cell's, `Clone::clone(&j)` of an
`I: Clone`, and past a drop that runs code. Which of them move an elision
is step 3's to measure, against the corpus's JS.

Of the keeps, none is a write the query can't see:

- 143 where a borrow is made, `assign ref mut`, which writes nothing: the
  rules forget an integer lent `&mut` there (`lent_where_borrowed`) or a
  `Vec`'s push (`unpush`); the query forgets it at the write the borrow
  makes, `*p = 0` (#1782's).
- 31 at a std call that runs no code and is given nothing of the cell it's
  of, `Vec::push` of another `Vec`, `Rc::clone`, `Arguments::from_str`,
  `Index::index` of a map: the rules forget each cell's length at any call
  that doesn't read one (`may_write_cells`).

The shadow found its own holes first, each now a mutation: a closure that
keeps the `&mut` it captures, a static's address, `NonNull::from`, a
`RefMut`'s writes, a cell a parameter's field holds read as a local, and a
`Box`'s cell a closure captures.

## Consequences

Step 3 makes the transfer forget each stale local's facts
(`State::forget_all_about`) and re-gen what std's modelled calls prove,
deletes the shape rules, and measures the elisions it moves against the
corpus's JS; the differential net (`--without-intervals`, ADR 1137
amended) runs every case without the intervals. Step 4 holds no `forget*`
outside the transfer by an architecture test.

Held by `test/intervals-shadow.test.ts`' "the effects query keeps nothing
the forgetting rules forget at a write", over the C3 cases, a `RefMut`'s
writes, a parameter's cell field and a `Box`'s cell, and mutations
`effects-call-result-shallow`, `effects-rewritten-locals-unescaped`,
`effects-constants-point-nowhere`, `effects-closure-keeps-nothing`,
`effects-borrows-by-lifetime-hold-nothing`, `effects-box-holds-nothing`,
`effects-nonnull-holds-nothing`, `effects-shared-cells-unwritable` and
`effects-indirect-writes-unborrowed`, each caught.
