# 1150. A variable is an id; its spelling is preparation's

Status: Accepted (2026-11-14, #1681's change 3, M3, the plan the lead
approved; in progress, a step a commit, each recorded here).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about how
rust-js's JS tree names its variables. No output changes: each step is
byte for byte what was printed before.

## Context

A variable of the JS tree was its spelling, a `String`: `ExprKind::Var`,
a `Const`'s and a `Let`'s name, a parameter's, a label's. Two variables
spelled alike were taken for one, and a pass that asked which variable
it had read it off the spelling: tidy's kinship, `b$1` given apart from
`b`, and preparation's reclaiming of `b$1` to `b`. M2 (ADR 1145) made each
compiler-made name unique in its item, the last of them not spelled yet
till preparation; what's left, a tree used again where it's needed twice
and the names tidy and three functions of no lowering make, needs a name
that's not its spelling.

## Decision

A name is `js::Name` (`src/js/name.rs`), the one type a read and a
declaration have, and its identity is its own: an id of its item's names,
of a global, or of a label, with its word, spelled once by preparation.
It's reached in eight steps, each byte-identical:

1. **S1, the accessor.** `Name` takes the place of the `String`s, one
   form for now, its spelling. S1a puts it in the tree with its word
   readable as before, `Deref` to a `String`; S1b takes that away a part
   at a time, each site then asking `Name`'s API.
2. **S2**, compiler-made names are ids (M2's `value#1` gone).
3. **S3**, a global is a form of its own, not a fact a read has.
4. **S4**, the program's locals are ids; tidy's kinship asks the table.
5. **S5**, labels are ids.
6. **S6**, tidy's own names are ids.
7. **S7**, a tree used again is given binders of its own; a module that
   declares an id twice is refused.
8. **S8**, a spelled name is preparation's alone; `Var(String)` is gone.

Then M4, the statement facts, and M5, the node-keyed forms, finish
change 3.

## Consequences

Follow-ups the lead recorded, outside S4, which keeps tidy's answers as
they are: once names are ids, tidy can ask whether two names are one Rust
variable rather than whether they're spelled alike, and tell a program's
names (`Local`) from the compiler's (`Made`).

## S1a (2026-11-14)

`ExprKind::Var(String)` is `ExprKind::Var(Name)`; `Name` derefs to its
word, a `String`, so every pass reads it as before, and each site that
gave or took a `String` converts, 53 files, no logic changed. The corpus,
the snapshots and the port are as they were.

Mutation `wrap-call-an-effect` asked `value_keys.rs`, whose 64-bit
constant is folded now (216a72342), so no `BigInt.asIntN` was left in it
and the mutation survived main's run; it asks `stock_orders.rs` and
`byte_constants_large.rs`, whose wraps are held where they'd be read
after what else they do, and is caught.
