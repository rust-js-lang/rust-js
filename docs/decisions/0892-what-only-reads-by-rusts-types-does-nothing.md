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

## Amendment (2026-11-22): a Rust number's conversion runs nothing (#1468)

`Number(n)` and `BigInt(n)` of a Rust integer, BigInt or `bool`, an `as`
cast's, `From`'s, a hash's or a shift count's, are JS's own conversions
of a number, which call no `valueOf` and throw nothing; `Number(obj)` of
an object would call its `valueOf`, so a call of `Number` alone says
nothing. Each is a node of its own, `ExprKind::Converted`, made only by
`Expr::converted` and printed as the call, as `Stringed` is `String(x)`:
`has_effects` reads it by its kind as what its operand does, and the
walks see through it as they see through `Stringed`. A node, not a fact
on a call: facts riding JS nodes are being retired (the lead's decision).
An arrow that converts its one parameter, `(n) => BigInt(n)` of
`u64::from` given as a function, passes it on to the global, `BigInt`, as
it did as a call (`passed_on::passed_to`).

So a value made of one is read where it's used, not held first, and no
`try` guards it: `tally.bytes + BigInt(bytes.length)` in place. ADR 0262
case: none, as nothing runs or throws either way. Nine corpus snapshots
and std's `io.js` move, each read once in the order it was. Mutations
`converted-has-effects` and `conversion-arrow-not-passed-on`.

The issue's own example, `range_defaults`' chain collected through
`$roomForHint`, no longer takes that path for the conversion: its chain
is std's compiled `Chain` since the iterator family's flip, whose collect
asks its hint as std's `SpecFromIter` does, so it stays.
