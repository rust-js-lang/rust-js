# 1199. One shadow flag, `--shadow <kind>[,<kind>]=<file>`

Status: Accepted (2026-11-24, #2490, the lead's call on the design).
Amends [0364](0364-mir-lowering.md) (`--drops-shadow`) and
[1137](1137-the-bounds-analysis-on-mir-shadow-first.md)
(`--intervals-shadow`).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A shadow is a
measurement; no program's JS changes.

## Context

A shadow measures an analysis before it decides anything. The compile
takes today's answer, and the shadow writes, of each site, the answer the
new analysis gives beside it. MIR's intervals were measured so
(`--intervals-shadow`, #1672), and the drop elaboration's comparison with
rustc's (`--drops-shadow`, #2112's change D). Each had its own flag, its
own field of the session's caches, its own report function and its own
write in the driver. Campaign 4 (#2490) needs a third, of `CellForm`'s
proof of a `RefCell` borrow's form. A third flag would be a third copy of
the same plumbing, which AGENTS.md's one-way rule rules out.

## Decision

- **One flag**, `--shadow <kind>[,<kind>]=<file>`, of the kinds
  `session::ShadowKind` names: `intervals` and `drops`, and each new one
  as it comes.
- **One owner**, `session/shadows.rs`. The session keeps one `Shadows` of
  the kinds asked. An analysis asks `shadowing(tcx, kind)` and adds a line
  with `shadow(tcx, kind, ..)`, which makes it only where the kind is
  recorded. The driver writes `shadow_report`: every line
  `<kind>\t<line>`, the kinds in `ShadowKind::ALL`'s order, each kind's
  lines once, in the order first given.
- **The old flags are refused**, `--intervals-shadow is gone: \`--shadow
  intervals=<file>\` records it`, and so is a kind there's none of.
- **A Cargo build's crates** are given the same flag by Cargo's wrapper:
  `RUST_JS_SHADOW=<kind>[,<kind>]=<dir>` becomes
  `--shadow <kind>[,<kind>]=<dir>/<crate>.txt` for each crate it compiles,
  as `RUST_JS_TEST_BUILD` becomes `--tested`. It is the wrapper's
  transport of the one flag, never a second interface: only `cargo.rs`
  reads it, never the driver. It is how a port's crates are measured,
  React's for #2490.

## Consequences

- No output changes: each shadow's lines are what they were, after their
  kind.
- The tests and scripts that read a shadow read it by its kind
  (`shadowLines`, test/support.ts).
- A new shadow is a `ShadowKind`, its name and its lines.
- Held by test/drops-shadow.test.ts' refusal of the old flags and of an
  unknown kind, test/cargo.test.ts' "RUST_JS_SHADOW gives a Cargo build's
  crate the lines --shadow gives it", and the intervals' and drops'
  shadows' own tests, which read the lines by kind.

## Amendment: the `cell-form` kind (2026-11-24, #2490)

Campaign 4's phase 1 is the `cell-form` shadow: of each `RefCell` borrow
of the crate's bodies, `site` and the form the compile takes (`unchecked`,
`momentary`, `counted`, or `declared`, a flattened state's), beside the
form `CellForm`'s proof gives from MIR, alone and with the crate's bodies'
borrow summaries (`CellBorrows::call_asks`), the borrow's shape in its
source, what it's borrowed of and its field, the first defeat of each, its
function and where; and `whole`, each field some body uses whole and how.
`analysis/cell_form.rs` proves; `decisions::record("cell-form", ..)` counts
`agrees`, `gains-count` and `loses-count`. No output changes: the table
`scripts/cell-form-shadow.ts` makes of the corpus's three builds and the
React port's crates goes to Tu before any form flips. Held by
test/cell-form-shadow.test.ts and the mutations of `cell_form.rs` and
`cell_borrows/unproven.rs`.

## Amendment: phase 2, what the proof reads (2026-11-25, #2490)

Phase 1's table left the React port 122 borrows the flip would count that
no run can find borrowed, 74 of them "unseen": a binding's or another
crate's code run while the guard's held. Phase 2 teaches the proof what
today's deciders read, each as a rule of its own, and measures again. No
output changes; nothing flips before Tu sees the table.

- **A call's arguments.** The proof read each call in a guard's range of
  its MIR alone, with no arguments, so a binding's method was any code.
  Each MIR call's THIR call is found by where its callee is written (one
  call there, or none found), and its receiver is given to `called`, as
  lowering's `Quiet` gives it: a method of what a builtin made (`AtHand`,
  `Map::get` of `js::map::new`), a binding that runs only what it's given,
  and a promise's resolve given what runs nothing.
- **Tu's exceptions 39 and 44, named, never proofs.** A guard that's only a
  call's receiver is held while the call runs, not while its other
  arguments do (`receiver_only`: a temporary guard, each way from its
  `Deref` through the arguments, an `if`'s or a `match`'s arms but no
  loop, coming to the one call it's the receiver of, which its drop
  follows); of the arguments, only what rust-js sees asks. A guard read
  only for a binding's argument isn't held while the binding runs. These
  are ADR 0262's differences 39 and 44, Tu's decisions (ADR 0863,
  amended): the proof keeps them as today's output does, neither wider nor
  narrower, and a site they pass is written with the difference's number,
  the line's last column, so the table counts what rests on an exception
  apart from what's proven.
- **Whole uses, proofs A and B.** A field used whole had every borrow
  counted. Proof A, a field lent only to parameters only ever borrowed,
  and proof B, a field of a type of no parameters none of whose borrows'
  ranges is loud, are the proof's now, B found once of the crate's bodies
  (`TypeProofs`, a session cache, as the shadow's alone needs it). Where
  neither passes and the range is quiet, the borrow is `momentary`,
  checked and not counted: a whole use needs the cell's flag, not a count.
  A declared form has no flag, so the proof fails it.
- **#659's and #687's rules, Tu's proofs.** A cell's own `swap` borrows
  each cell, so it's no whole use. A flattened state's `try_borrow` is
  proved by its range, as every borrow of the state is proved quiet or
  refused. Any other `try_borrow` is counted by design, but one of a type
  no guard of which is loud, which nothing can find borrowed (ADR 0863's
  "a `RefCell` no borrow of which can meet another"). A thread-local's
  `with_borrow(_mut)` and a cell's `replace_with` hold their cell while
  code runs, so their types are loud, and a loud type of a generic body is
  loud of each instance.
- **Two fields' places are two cells.** A borrow, or a cell's own call
  that runs none of the crate's code, of another field than the guard's
  never meets it, whichever objects they're of.
- **Drops.** A type parameter's value dropped while a guard's held runs
  each instance's destructor, of the instances its callers make, any where
  they aren't all seen (#1321). A std type's drop runs what it holds, an
  `Rc<Loud>`'s `Loud`'s, and a struct's its fields'. A cell's own call
  reads what it runs by its bounds, a `with_borrow_mut`'s closure.

Measured, each "corpus (binary) / port", quiet today 1,233 / 2,327:

| | phase 1 | phase 2 |
|---|---|---|
| gain `$borrow`, with summaries | 50 / 122 | 14 / 3 |
| held by exception 39, not proven | | 0 / 63 |
| held by exception 44, not proven | | 0 / 4 |
| unchecked today, gain a check | | 0 / 0 |
| a field used whole, passed by proof A; by B alone | measured, not taken: 12, 13 / 14, 8 | 12, 13 / 12, 18 |
| declared, refused | 16 / 1 | 0 / 0 |
| counted today, proof quiet | 53 / 10 | 50 / 15 |

What's left: 13 corpus and 2 port guards held while a type parameter's
value drops, whose instances aren't all seen (generic code a library's
consumer may give any type: today's `drop_may_ask` answers them quiet),
and the "contains-guarded" push of `contains_guarded_borrow.rs` and the
port's `ReactFiberGestureScheduler.rs:132`, which only a path's test
tells apart. A counted borrow the proof passes loses its count at a flip:
#2491's count-every-cell net and #2481's observe rewrites hold those.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a shadow changes
no output. Held by test/cell-form-shadow.test.ts and the mutations of
`cell_form.rs`, `cell_borrows.rs` and `cell_borrows/unproven.rs`.

## Amendment: the `param-form` kind (2026-11-25, #2539)

Campaign 6's step 0 adds the `param-form` shadow ([1203](1203-a-generic-instance-s-view-of-its-type-parameters-is-one-record.md)).
Its `site` lines give, for each value converted where it crosses into or
out of generic code, the form taken beside every form that differs there.
Its `iterated` lines give, for each answer of whether generic code holds
an iterator as it is, the bounds that read its value.
`bun scripts/param-form-shadow.ts` counts them over the corpus or a
port's files.
