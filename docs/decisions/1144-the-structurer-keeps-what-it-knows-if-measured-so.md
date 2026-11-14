# 1144. The structurer keeps what it knows only if a week's measurement says so; its kill criterion is written first

Status: Measuring (2026-11-14, #1671's change 5; the criterion the lead
approved, written before any run; decided by 2026-11-21).

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this is how
rust-js's JS is shaped, not a difference. Behaviour must not change.

## Context

`mir/structure.rs` makes a body's control flow structured JS by Ramsey's
"Beyond Relooper": every loop is `bbN: while (true)`, every join a labelled
block, every branch's end a jump. FalseEdge and FalseUnwind, rustc's own
marks of a match candidate and of a `loop` the program wrote, are
collapsed to plain edges (`cfg.rs`). Tidy's control-shape stages then
rebuild what was dropped from the JS: `jumps` and `jumps again` (16 passes
each), `loops` (12) and `logicals` (5) of its 323 passes.

Review #1671 ruled against a new body representation and against lowering
from THIR. It left one question open: should the structurer keep what it
already knows (whether a jump only falls through, and rustc's marks)
instead of tidy recovering it? Its answer was to measure for a week, with
the kill criterion written down before running. This ADR is that record,
written before the first run. The results and the decision are appended
here when the week ends.

## Decision

**The arms.** These sit behind one development flag, `--structure-keeps`,
which is removed at the week's end whatever the outcome:

- **(A) Fall-through.** No jump is emitted where its target is what
  follows in tail position: a `break` to the label that ends right after
  it, or a `continue` at the end of its loop's body.
- **(B) rustc's marks.** FalseEdge and FalseUnwind are carried through
  `cfg.rs` as edge marks the structurer reads (a `loop` the program
  wrote, a match candidate's order), not as plain edges.
- **(C) No THIR hints.** A separate switch, `--no-thir-hints`, empties
  `tidy::Hints`.

**What's measured.** Each arm is run over the corpus in both builds, the
React port, alien-signals and 600 generated programs. For each run:

- native equality;
- the share of files whose output is byte-identical;
- per pass, the bodies changed in the control-shape stages
  (`scripts/tidy-fired.ts`);
- the label census rows (`scripts/label-census.ts`).

**The kill criterion.** The structurer goes on to own fall-through and
the marks, with tidy's passes that stop firing deleted, only if A and B
together meet all four of these:

- **k1.** They add no wrong answer that can't be fixed inside the
  structurer. Every native difference is fixed there, or the arm is
  killed.
- **k2.** They remove at least a third of the control-shape stages'
  summed firings (bodies changed). Alternatively, at least 5 of those
  passes fire on no body across the corpus, the ports and the programs,
  so they can be deleted.
- **k3.** No output reads worse: no added label, helper or
  `while (true)`, and no expression form lost. Any file whose output
  changes has fewer statements or labels.
- **k4.** They add at most 300 lines to `structure.rs` and `cfg.rs`.

Otherwise the measurement is killed. The flag and its code are deleted,
the numbers are recorded here as measured and rejected, and tidy stays
the owner of control shape.

**(C) is not a kill input.** The hints are readability alone, so emptying
them must change no behaviour. A native difference under (C) means a
semantic fact is travelling through a hint. It is filed as its own P0 and
fixed whatever the measurement decides.

## Consequences

- No change to output or to the guide until the decision is made. The
  flags are development-only and undocumented in the guide. The arms'
  code lives beside what it replaces, in `structure.rs` and `cfg.rs`.
- If the measurement passes, the migration is its own change: the
  structurer owns fall-through and the marks, and the passes that stop
  firing are deleted, each with its mutation retired. That change is
  recorded here as an amendment, with the guide updated if output changes.
- If it is killed, the review's change 5 is closed with these numbers,
  and nobody reopens it without new evidence against this criterion.
