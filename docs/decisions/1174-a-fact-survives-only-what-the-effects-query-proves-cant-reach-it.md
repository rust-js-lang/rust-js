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

## Amendment (2026-11-19, step 3a): what a write does, and the transfer that asks the query

The transfer asks the query now behind a test hook, `--intervals-by-effects`
(`intervals/transfer.rs`): the forgetting rules first, then what the query
says each stale local's write does. Measured against the rules alone over
the corpus's 2,250 proofs (`proved-*` lines of `--intervals-shadow`), a
query that forgot each written local whole lost 200 (8.9%), none gained.
So a write says what it does to what it reaches (`Written`):

- **Replaced** (`Whole`): a `&mut`, a raw pointer, a `Box`, of the type it
  points to; or a cell, `RefCell<_>`, `Rc<RefCell<_>>`, however it's
  reached: nothing of it known.
- **Its items** (`Items`): a reference is typed, so one to what isn't of its
  type, an item or a slice of a `Vec`, `&mut [T]` of `deref_mut`, changes
  its items, never its length; nor is a slice's, an array's or a `str`'s
  length changed by anything. A table's rows' lengths are forgotten, and
  of a scalar written through an item, `board[r][c] = x`, nothing.
- **Grown** (`Grows`): a `Vec`'s `push`, std's, what its signature can't
  say: `x < len` and its least hold, its exact length doesn't.
- **Nothing**, through a shared reference to what holds a cell but isn't
  one, `&Vec<Rc<RefCell<_>>>`: no length changes.

What std's signatures can't say is named in `effects.rs` alone: the reads
`may_write_cells` listed (`reads_only`, which it asks now) and the
pointers `keeps_vec_length` gives (moved here from `recognition`), with a
`RefCell`'s `borrow_mut()`. A pointer a call of code not std's gives back
may point to what's escaped; a pointer's own pointee is seeded only of a
pointer, a parameter or a local the rewrites made.

Measured again: 38 of 2,250 lost (1.7%) in 25 cases, none gained. About 22
are a `RefCell` guard held while code runs (`take_first`'s `f(items[i])`,
`position_index_of`, the undo stacks): the rules' `guards` know no other
borrow can reach a cell a held `RefMut` guards; the query doesn't. The rest
are single cases. Step 3b, the rules deleted, follows the lead's decision
on the guards.

Held by "the effects query keeps what the rules prove of what no write
reaches", seven cases each of a class the query once forgot, and "the
effects shadow says where the query keeps what the rules forget"; mutations
`effects-replaced-taken-for-items` (the unsound direction, caught by the
keeps row) and `effects-items-taken-for-replaced` (the lossy one, caught by
the proofs row), `effects-references-untyped`,
`effects-scalar-store-resizes-rows`, `effects-push-replaces`,
`effects-shadow-keeps-unsaid`, each caught.

## Amendment (2026-11-19, step 3a): a held guard, as `cell_borrows` counts it

A `RefCell` guard held across code, from its `borrow()` or `borrow_mut()`
till its drop, a move or its storage's end on every way there, covers
what it points to: no code run meanwhile writes it but through the guard,
as Rust's second borrow panics (`effects/guards.rs`). In JS that holds
where `cell_borrows` counts the guard, where code run under it may borrow
its cell, unnamed code too (`call_asks` of what can't be named,
`anything_asks`), so a borrow it makes panics as Rust's does, or has proved
none does. The lead's condition: it's asked of `cell_borrows`
(`CellBorrows::unnamed_may_borrow`, in `cell_borrows/unnamed.rs`), not
re-derived, and where neither holds, a guard made in a call's own
arguments, which counts only what it sees (`call_asks_seen`), across code
that may be unnamed and may borrow its cell, the query's answer stands.

Measured: 31 of 2,252 lost (1.4%) in 23 cases, none gained (the two
proofs more are of the new case). The guard covers what a guard reaches
one pointer down; about 15 of the 31 are still a cell reached past it, or
a temporary guard across a `dyn`'s code.

Held by corpus case `refcell_written_through_a_clone_under_its_guard`,
code that clears the cell through another `Rc` under a `borrow()`, a
`borrow_mut()`, an `impl Fn`'s and a `&dyn Fn`'s, and past a dropped
guard, run against native; "a guard held across code that writes its cell
through a clone runs as Rust does, asking the query", its JS and
`refcell_guard_bounds`' with `--intervals-by-effects` against native; "a
guard keeps what it covers only while it's held"; the shadow saying a keep
`under a guard` where the query without guards would forget it; mutation
`effects-guards-cover-nothing`. A guard kept past its drop shows only once
the rules are gone, where they forget first now: step 3b's native row
holds it.

## Amendment (2026-11-19, #2112 B 3b): the query's answer is the targets', never their order

Found measuring step 3b's cold compile: a second form of `Targets`, dense
over a body's targets, iterated them in local order where the `IndexMap`
iterated them in insertion order, and 7 of the React port's 7,942 bodies
then had other `Effects`, compared field by field, sorted. A monotone
dataflow over a finite lattice reaches one least fixed point whatever
the order, so a join wasn't one: `Flow::reached_by`, what code given a
place may write, took each target's cap by `insert`, an overwrite, after
an earlier target's `reachable` may have reached it with more. The later
way lowered it: `ReactFlightClient`'s `trigger_error_on_chunk`, a call's
`written` at one block, was `Pointee(_30)` whole and nothing more in one
order, `_26` to `_30` items and `_34`, `_35`, `_48` whole in the other.
A lowered cap reads as a write that does less, so a fact could survive a
write it should forget: a wrong answer, latent, none found in a program
(the corpus, the snapshots and the React port print the same JS either
way, and a short try at a program of its shape found none).

`reached_from` joins, each target's cap the most of every way to it, and
is a function of the targets alone, `reachable` given. With it, the two
forms' `Effects` are equal for each of the port's 11,138 builds (each
written in one write, so concurrent compiles' records don't interleave).
A unit test runs it over the same targets in both orders,
`lower::analysis::effects::tests::what_code_reaches_is_the_targets_never_their_order`;
mutation `reached-cap-overwritten`, the `insert` back, is caught by it.
