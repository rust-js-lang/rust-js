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

## Amendment: the count-every-cell net (2026-11-25, #2491)

`rust-js --count-every-cell`, a test hook as `--without-intervals` is,
compiles every `RefCell` borrow counted, whatever today's deciders prove.
Two roots do it. drops/facts.rs keeps no borrow momentary, and
analysis.rs' thread-local `quiet` answers no, so no thread-local is its
module's variable. Between them, every field's, item's and local's borrow
takes the counted form: mutations removing the switch from
`RefCells::quiet`, `quiet_field` and `quiet_class` changed no JS, so it
lives only at those two roots. A declared form (a class's flattened
state, ADR 1044, or an exported thread-local's `export let`) keeps its
declaration and is refused counted, which the nets accept: no JS is made
to be wrong.

Two nets run it. test/tidy-stages.test.ts' `cells` family runs its few
cases per push of a change to `cellsPaths` (scripts/iteratorSlice.ts),
and every case nightly. test/metamorphic.test.ts' `observe-borrows` gets a
third build, `cells`. With a flag at every cell, it holds each flag the
probes print after each statement to native Rust's at every site, those
today's deciders leave unchecked included. That is the flip's own safety
check: the sites a flip quiets are held counted here. The builds are one
catalog, test/builds.ts' `buildFlags`, which the reducer and the
metamorphic net read.

First sweep, of 2,966 cases: 2,936 run as native Rust does, 29 declared
forms are refused counted, and 1 runs wrongly: #2579. A counted guard's
`let Some(x) = cell.borrow().clone() else` read the guard's value after
`$unborrow` and a write. observe-borrows' 1,145 variants in three builds
found only that case. Case ([0262](0262-when-rust-and-js-disagree.md)):
none; a test hook changes no output.

## Amendment: the flip, `CellForm` decides each borrow's count (2026-11-25, #2490)

Tu's decision was "flip after two checks", and both passed. First, the 3
variants observe-borrows refused at the 14 sites the flip counts were one
case, refused in every build by its probe's generic `impl Observe for
Rc<T>` (ADR 0852's wildcard and an untagged enum). A copy without
`untagged` compiled and ran as native. Second, the port's 15 quieted
sites, read against upstream React ab0d927, are borrows Rust can't
contest.

**One decider.** `CellForms` (`analysis/cell_form.rs`), proved once per
crate and kept in the session's caches, answers whether each `RefCell`
`borrow`, `borrow_mut` and `try_borrow` is counted. Each borrow is keyed
by its body and the place its callee is written. Where two borrows are
written at one place, the quieter answer holds only if each has it. A
borrow is counted unless its guard's range is proved quiet with the
summaries. A borrow proved quiet is checked where it's made (momentary),
or left unchecked where no borrow of its cell can meet another. THIR's
facts (`drops/facts.rs`) carry these answers and decide nothing. THIR's
old decider is deleted whole, with its mutations: ADR 0328's momentary
walk, ADR 0863's `let_held`, `Quiet` and `Seen`, and `ended_before`.
`facts.rs` keeps what isn't a `RefCell` decision: what's refused of a
body, and a `replace_with`'s momentary borrow.

**What the proof reads, beyond phase 2.**
- (a) A type parameter's value dropped while a guard's held asks
  nothing where the body is given no drop of it that may ask
  (`Proving::given_drops`, the `DropQuery` the body's lowering uses).
- (b) ADR 0863's case N, a borrow that can't meet the loop's guard, is
  read from THIR's shapes (`analysis/cell_form/shapes.rs`, `Shapes::unmet`),
  asked where a second borrow is.
- (c) A property read (#2113). A field's read or write, in the range, of a
  struct of the program's or a binding's is JS's property access, which a
  getter or a setter may answer. It's found by the MIR place's field
  projection and its type (`CellBorrows::property_access_asks`), the
  defeat `property-access`. A part of std's own, such as the `Box`
  pointer MIR writes for `vec![x]`, isn't such a read. THIR's question of
  a value at hand isn't asked here, so this side is the conservative one.
- (d) Whether code rust-js can't see runs in the range, which a library
  that borrows the cell unchecked refuses (ADR 0863), is carried per
  borrow (`CellForms::seen`). A range is unseen where a call in it is one
  `call_runs_unseen` or `pointer_call_runs_unseen` says may run unseen
  code (a promise's resolve given what runs nothing excepted), or where
  it suspends, runs assembly or tail-calls.
- A flattened state's whole use defeats its borrows alone, never with the
  summaries (Tu): its whole uses are its declaration's to refuse or keep.

**Five proof bugs, found by running on the flipped compiler.**
The corpus found four that no net had seen, and the suite found the
fifth, (c):
1. A moved-from guard's `Drop` ended the range (`refcell_let_guard_kept`).
2. `mem::drop(v)` of another value ran no destructor
   (`guard_ended_before_its_value_is_dropped_by_the_caller`).
3. A `map_split` half's drop ended the range, and `Ref::map`'s borrow
   stays counted, since its mapped guard releases a count (#1275,
   `ref_cell_guards_mapped`).
4. A `Result`'s or an `Option`'s payload move is a move-out only where
   what moves holds a guard (`flattened_state_tried`'s `Err` arm).

**Moves.**
- Corpus: 31 cases lose counts and none gain. `rc_dyn_listeners`,
  `rc_refcell_dyn_shared` and `trie_of_counted_rcs` now differ as a
  library (a `.program.js` each), and `unsized_struct_fn` no longer does.
- A borrow made only to be checked, its guard dropped unread, is its check
  alone. Before, it left its cell behind as a statement, `c;`
  (`borrow_checks_first`). This replaces #1815's "stays a count".
- Tests: lowering-cells' `ticked` is no longer counted, since its `Tick`'s
  destructor sets only `ticks`. borrow-forms' unread guard is its check.
- The React port: 10 files, only removals of `$borrow`/`$already*`.
  BusboyReading's `head = current.next;` stays after the `catch`, as
  upstream writes it (#2591, `prepare/catches.rs`).

**A shared guard's value, read as its cell.** A counted guard is a local,
`const borrow = $borrow(cell)`, and tidy read each `borrow.value` where
it's read. Uncounted, the guard is the cell itself, so its `Deref`'s
value became a temporary that the cell's own `{ value }` key named:
`const value = cell; $ptr(value[2], ..)`. `QuietCells::guarded`
(`mir/local_facts.rs`) marks each `Deref` of a shared guard whose borrow
isn't counted. `MirCx::temporaries_names` passes the marks to tidy as
`TemporaryNames::guarded`, and `readable_temporaries` reads such a
temporary as the cell where it's read, `$ptr(cell[2], "a", 4, cell, 20)`,
where it reads the same and the program names nothing there. A
`Ref`'s value can't change while the `Ref` lives. A mutable guard's is
left as it was, the safe default; #2593 is to narrow that.

**Readability.** The flip removes 483 lines of the corpus's and the
snapshots' JS, `$borrow`, `$unborrow` and their `try`/`finally`.
Counted like for like, renames fall from 18,343 to 18,330 and role
fallbacks from 10,214 to 10,212. Their shares in 10,000 lines rise only
by the denominator, 424.65 to 424.83 and 236.46 to 236.68, and
docs/readability.txt is blessed for that alone. Two files still gain one:
- address_of_a_nested_field_through_a_deref: `const value$1 =
  $storedValue(cell, 4).value;`. A guard's value read through the
  address-keeping helper doesn't read the same each time, and the source,
  `&raw const (*cell.borrow())[2].a`, names nothing.
- iterator_in_a_ref_cell_given_generic_code: a mutable guard's, #2593.

The `+2`/`+2` of guard_value_read_before_write are #2579's new
functions, not the flip's.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A count that
nothing can ask about changes no result, so JavaScript's form, no count,
is taken. Where the proof can't show that nothing asks, the count stays.

Held by:
- test/cell-form-shadow.test.ts, its flattened whole-use fixture among
  them;
- test/builtins-runs-nothing.test.ts' #2113 rows;
- the corpus's `flattened_state_tried_guard_kept_past_its_result_refused`
  (a payload's move followed past its `Result`'s drop),
  `guard_held_while_a_type_parameters_quiet_value_drops` (each instance's
  destructor asked, `drops_given` the one answer of what a body is given)
  and `replace_with_borrowing_another_cell` (a `replace_with`'s cell by
  its flow's class);
- test/crates.test.ts' library refusal;
- the count-every-cell and observe-borrows nets;
- the mutations of `cell_form.rs`, `cell_form/shapes.rs`, `facts.rs`,
  `cell_borrows.rs`, `borrow_checks.rs` and `catches.rs`.

## Amendment: the `param-form` kind (2026-11-25, #2539)

Campaign 6's step 0 adds the `param-form` shadow ([1203](1203-a-generic-instance-s-view-of-its-type-parameters-is-one-record.md)).
Its `site` lines give, for each value converted where it crosses into or
out of generic code, the form taken beside every form that differs there.
Its `iterated` lines give, for each answer of whether generic code holds
an iterator as it is, the bounds that read its value.
`bun scripts/param-form-shadow.ts` counts them over the corpus or a
port's files.
