# 0902. What never unwinds is read from MIR

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. The review's P2: whether a
node can unwind, computed once from MIR and carried on the JS tree.
Extends [0901](0901-a-print-throws-only-where-its-text-does.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. A `try`
only goes where nothing in it can throw; what runs, and in what order, is
the same.

## Context

After ADR 0901, a print of numbers and text throws only where its text
does. A call of the crate's own function was still one that may throw.
So `loudDrop_drop(b)`, a `Drop` that only prints, put the drops after
it in a `try`/`finally`, even though nothing in it can panic.
`may_throw` also took a template or an array that held any call as
throwing, without asking its parts.

## Decision

- **`never_unwinding` (`analysis/unwinding.rs`) finds, from all the
  crate's non-generic functions down, those whose every block can't
  panic.** A block's terminator must be a jump, a test, a return, or a
  call of another such function, or of std's `_print`/`_eprint`, its
  `Arguments`, or an `Argument`'s `Display`/`Debug` of a number, a
  `bool`, a `char` or text. Never an assert (an index's bound or a
  divisor's), a drop, nor what's unreachable. Overflow checks are off,
  as release Rust's (ADR 0011). Cleanup blocks don't count.
- **A reference to one is marked `Known::NeverThrows`** where every
  reference is made (`fn_ref`). `may_throw` of a call of it is its
  arguments'.
- **`may_throw` of a template, an array, a binary operation or a
  condition is its parts'**, as `has_effects` reads them:
  `` `${total(x)}` `` throws where `total(x)` does.
- What it exposed, each fixed:
  - **A temporary made by the program's code is made before a call
    that unwinds elsewhere**, as one that may throw is
    (`may_throw_first`). `slot.pending.insert(made(5))` dropped the old
    value before `made(5)` ran, printing `drop 4` before `made 5`
    (`option_destructors.rs`).
  - **`stmts_alike` ignores what's known of each part**, as it does
    where each is written: the same JS, one copy of it marked.
  - **A temporary's new name isn't given to a closure's own parameter of
    its name** (`js::outside_shadows`). Renaming `_2` to `value` renamed
    `(_2) => _2[0]` too, so the closure read the outer value
    (`temporaries_taken_apart.rs`).

## Consequences

- 52 snapshots change: 43 `finally`s and 147 `catch`es go, 1,718
  lines fewer. One reads worse, `given_last_unwinds.js` (+12 lines): a
  drop inside its loop no longer needs its own `try`, so the scope
  forms inside the loop and no longer wraps it. That's correct, and
  left for a `Run::covers` that takes a `try`/`finally` in.
- The readability baseline is blessed: each shape's share grows 1 to
  2% as the lines around it go (`$iter` 84.8 to 86.3, `renamed x$1`
  727.9 to 731.5); no construct is new.
- `never_unwinding_calls.rs` holds both sides against native Rust: a
  print-only function needs no `try`, and an index past the end or a
  panicking `Display` still drops what's held.
- Mutations: `assert-steps-on`, `user-display-shown-whole`,
  `never-unwinding-unmarked`, `template-may-throw-whole`,
  `program-code-pending-unflushed`, `shadowed-temporary-renamed`.
