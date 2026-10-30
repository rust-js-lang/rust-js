# 0901. A print throws only where its text does

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. The review's P2: whether a
node can unwind, a fact of Rust's on the JS tree, not read back from its
JS shape. Builds on [0132](0132-std-odds.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. What runs,
and in what order, is the same; a `try` whose body can't throw runs its
`finally` once, after it, as the statements after it would.

## Context

`Expr::may_throw` took every call but a few runtime helpers' as one that
may throw. A `println!` is `console.log(text)`, a call, so each
`println!` while a value with a destructor was live was a statement that
may unwind. Its drops were a `try { .. } finally { drop(x); }`, or a
`try { console.log(..); } catch (error) { drop(x); throw error; }` per
print. Rust's `println!` panics only when the write fails, and ADR 0132
already decided a write to a standard stream never fails in JS, so those
guards guarded nothing. What the text is made of keeps its own answer: a
`Display` of the crate's is a call, which may throw.

## Decision

- **Std's print is marked `Known::NeverThrows` where it's lowered**
  (`format_args.rs`, a write's and `print!`'s `console.log`/`console.error`):
  a call that throws only where what it's given does. `$print` and
  `$eprint`, the same write of text that may not end a line, are in the
  runtime's `NEVER_THROW`.
- **`may_throw` of a call so marked is its arguments'**.
- **A drop is never a declaration** (`dropped_last`, `tidy/unwinds.rs`):
  once a print no longer counted as throwing, a `catch` left holding only
  `const dropMany = ..`, the drop function a dead call had used, was taken
  as the drop a scope ends with. Its `finally` scoped the name away from
  the code after it, which threw `dropMany$2 is not defined`
  (`drop_functions.rs`).

## Consequences

- 151 snapshots change, all for the better: 183 `finally`s and 222
  `catch`es go, 3,579 lines fewer. `refcell_borrows.js`' borrows end after
  their print, `console.log(..); $unborrow(..)`.
- The readability baseline is blessed with it: each shape's share grows 2
  to 3% as the lines around it go (`renamed x$1` 713.5 to 727.3, `$iter`
  81.9 to 84.7, `Iterator.from` 14.6 to 15.1, `while (true)` 10.1 to 10.4,
  `return;` 9.7 to 10.1); no construct is new.
- Left open: `try { tmp = f(); } catch (error) { throw error; }`, a
  `catch` whose drops a later stage found never run, as main's snapshots
  already had it (`drop_functions.js`).
- Mutations: `print-may-throw`, `declaration-taken-for-drop`.

## Amendment: no `catch` declares a drop function (2026-10-30)

Since a function lowered from its MIR declares each drop function once,
at its top (ADR 0098, amended), no `catch` holds `const dropMany = ..`,
and a drop taken for a scope's is never a declaration by construction:
`dropped_last`'s check that it isn't one decided nothing, its mutation
`declaration-taken-for-drop` surviving every check, the corpus's JS the
same without it. Both are gone; the hoisting is the rule's one owner.
