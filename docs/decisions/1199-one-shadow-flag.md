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

