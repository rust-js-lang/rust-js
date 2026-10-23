# 0609. A `let` that shadows a parameter read no more is the parameter assigned

Status: Accepted. Extends [0352](0352-reclaimed-names.md)'s reclaimed
names. Extended by [0650](0650-a-shadow-of-a-binding-read-no-more-is-that-binding.md):
any binding of the shadow's block, the program's own, is assigned; the
pass is tidy's, `tidy::shadowed`, not preparation's.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same values are
read; only the name changes.

## Context

Rust shadows a parameter with a `let` of the same name, `let
priority_level = match priority_level { .. };`. JS can't declare a name
its parameters have, so rust-js named the `let` apart, `priorityLevel$1`,
where React's scheduler assigns its parameter, `priorityLevel =
NormalPriority`.

## Decision

**A `let` at a function's top that shadows its parameter, which nothing
reads after it nor a closure made before it, is the parameter assigned**:
`level = level > 5 ? 3 : level;`, read by its name after. Of the
parameter itself, `let mut n = n;`, it's nothing. One a closure made
before reads keeps its own name, `level$1`: assigning the parameter
would change what the closure sees.

## What holds it

- `test/lowering.test.ts`, "a let that shadows a parameter read no more
  is the parameter assigned", with a closure made before; the `closures`
  snapshot's `let mut n = n;`.
- Mutations: the `let` kept apart, the parameter assigned under a
  closure, `n = n;` written.
