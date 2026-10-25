# 0892. What only reads, by Rust's types, does nothing

Status: Accepted. The review's P2: a fact of Rust's on the JS tree answers
the oracle, not a copy of it. Amends [0705](0705-a-guards-scope-reads-as-any-body.md)'s
test of a guarded loop.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. Each
call it covers can't throw or change anything, so the JS does what it did,
with fewer `try`s.

## Context

A `str`'s or a slice's method that only reads it, `s.startsWith(t)`,
`items.at(-1)`, `w.toUpperCase()`, is marked `Known::Read` where Rust's
types say it's one (`Expr::read`), never a crate's own `at`. Only
`iterators::does` knew that: its own copy of `has_effects` with one
exception. Everywhere else asked `has_effects`, which took the call to do
something. So a guard's scope wrapped it in its own `try`/`catch`, a
`finally` round a loop that could throw nothing, and an applied callback
stayed an IIFE where it could be a `const`.

## Decision

- **`has_effects` says it of a `Known::Read` call of one of `READS`'**:
  what its operands do. `does` asks `has_effects`, the one answer.
- **A guarded loop that only reads is no `try`**, and reads as it would
  unguarded. That includes ADR 0705's rule that a `while`'s test takes the
  `if` of a `break` first in its body, "one rule, not one per where the
  `if` came from". Its test of a guarded loop keeps a call that may throw,
  the crate's `is_root`, so it still holds a guard's scope. A new test,
  "a guarded loop that only reads is no try", holds this decision.

## Consequences

- Seven corpus programs read shorter: `allocator.rs`, `slice_edit_drops.rs`
  and five more lose a `try`/`catch` round a read that can't throw, or an
  argument spilled only because of it.
- The mutation `reads-by-name-alone` moves to `expr_walk.rs`, and
  `read-call-effectful` holds the new arm.
