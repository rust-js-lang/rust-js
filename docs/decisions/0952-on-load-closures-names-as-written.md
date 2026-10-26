# 0952. Names in the closures of code that runs at load are named as written

Status: Accepted. Extends [0357](0357-scoped-local-names.md), a local's
name given back where each read stays its own, from functions and
constants to the closures and blocks of an `on_load!` body
([0267](0267-on-load.md)), whose outermost names stay the module's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A name is how the JS
reads; each read reads what it did.

## Context

React's `ReactStrictModeWarnings` sets its functions where the module
loads, `js::on_load!` in its port. Its two `for (const fiber of ..)` loops
and two blocks' `const sortedNames` read `fiber$1`, `fiber$2` and
`sortedNames$1`: `scopes::Tree::of` gave no tree of an `on_load!` body,
whose names are the module's, so none of its locals was named back.

## Decision

**An `on_load!` body's scopes are walked as a function's**, its outermost
scope the module's: a name declared there, or a rename to one declared
there, is refused, as it's the module's; a name inside a closure or a
block of it is named back where each read stays its own, a loop's
`offset$1` kept where it reads the module's `offset`.

## What holds it

- `test/lowering-shadows.test.ts`, "names in the closures of code that
  runs at load are named as written".
- Mutation: `on-load-scopes-unwalked`.
