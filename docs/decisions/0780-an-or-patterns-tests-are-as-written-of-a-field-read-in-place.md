# 0780. An or-pattern's tests are in the order written, of a field read in place

Status: Accepted. Extends [0689](0689-a-moves-decision-is-apart-from-the-move.md)'s
moves: a temporary read more than once, by one statement's tests.
Amended (2026-10-25): an `if let`'s pattern and a reference's are read too,
and each test is where its alternative is written ([0444](0444-a-comment-is-where-its-code-is.md), amended).

Case ([0262](0262-when-rust-and-js-disagree.md)): N: which of an
or-pattern's pure tests runs first, and whether a field's read is held
or read again where nothing between can change it, are nothing a program
can observe. A getter's read, which can, stays held.

## Context

React's ReactLazy.js tests a lazy component's status:

```js
if (payload._status === Pending || payload._status === Uninitialized) {
```

Its port writes `matches!(payload.status.get(), Status::Pending |
Status::Uninitialized)`, which printed

```js
const _status = payload._status;
if (_status === -1 || _status === 0) {
```

Two things differ. MIR switches on an enum's discriminant with a target
of each variant, in the order the enum declares them, so the tests were
the enum's order, not the pattern's. And `Cell::get` is a MIR call whose
value is a temporary, read by each test, so it was held in a `const`.

## Decision

**An or-pattern of variants tests them in the order its arm's pattern
names them**, `Pending | Uninitialized`'s `=== Pending || ===
Uninitialized`, of a `match` and of `matches!` alike
(`FnCx::written_variants`, asked by `mir/structure.rs` of each target
more than one variant goes to). A macro's `match` is found where its call
is, the program's pattern its first arm. Variants the pattern doesn't
name, a `_`'s, stay in the enum's order.

**A temporary of a field's read, `payload.status`, that one statement
tests more than once, and nothing else reads, is read in each test**
(`tidy::moves::tested_in_place`): where the statement's expression does
nothing, each read a side of a `===` or `!==`, and the value a chain of
properties of a variable or `this` no getter answers (`js::Known::Getter`).
Nothing runs between the tests that could change what it reads; what
runs before the statement is `moved`'s question, as for any temporary.

```js
if (payload.status === "Pending" || payload.status === "Uninitialized") {
```

A call's value, and a getter's, which may do something or give another
value each time, stay held: `const state = thenable.state; return state
=== 2 || state === 1;`.

## Why

- **React's JS**, read line by line against upstream's.
- **The program's order**: the arm says which variant it's about first.

## Alternatives

- **Every temporary read twice, read in place where it's pure**: a long
  chain read in several statements would be read and printed many times;
  the tests of one or-pattern are where a person writes it so.
- **Sorting by the source text of the pattern**: THIR's pattern already
  says it, by variant.

## Consequences

- Snapshots whose or-patterns named variants out of declared order read
  them as written.

## What holds it

- `test/lowering-or-patterns.test.ts`: the order of `matches!` and of a
  `match`'s arm, a field read in each test, run; a call's value held and
  called once; a getter's value held and its getter called once.
- Mutations `or-pattern-declared-order` (`mir/structure.rs`),
  `or-pattern-macro-arm-unfound` (`mir/temporaries.rs`),
  `tested-field-held`, `tested-getter-in-place` (`mir/tidy/moves.rs`).

## Amended (2026-10-25): an `if let`'s, and a reference's

`written_variants` read a `match` arm's pattern alone, and of a
reference, `if let Shape::Line | Shape::Circle(_) = shape` of a `&Shape`,
saw match ergonomics' `&` and no alternative. **An `if let`'s pattern,
the innermost `let` its switch is in, and a reference's alternatives are
read too**, each with where it's written, which its test is placed at, a
comment before it before its operand (ADR 0444, amended). React's
cache's `Kind::Function(_) | Kind::Object(_) | Kind::Array(_)` was in
the enum's order already; a `Kind::Number(_) | Kind::String(_)` is
`typeof v === "number" || typeof v === "string"` now, as written. Held by
`test/lowering-comments.test.ts`, "a comment before an or-pattern's
alternative", and the mutations `if-let-alternatives-unwritten` and
`deref-alternatives-unwritten` (`mir/temporaries.ts`).
