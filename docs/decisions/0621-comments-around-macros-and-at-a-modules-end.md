# 0621. A comment before a macro's opening is its code's; one after a module's last code ends its JS

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments and blank
lines, which nothing runs.

## Context

A comment is its next statement's where no `;`, `{` or `}` of code is
between them (ADR 0444, rule 1). React's `ReactSymbols` has two that were
lost:

```rust
// ATTENTION
// When adding new symbols to this file,
// Please consider also adding to 'react-devtools-shared/src/backend/ReactSymbols'

thread_local! {
    // The Symbol used to tag the ReactElement-like types.
    pub static REACT_LEGACY_ELEMENT_TYPE: &'static Symbol = js::symbol::r#for("react.element");
    // ..
}

// ..

// This is actually a symbol but Flow doesn't support comparison of symbols to refine.
```

- **Before `thread_local! {`**: its `{` is between the comment and the
  first `static`, which is where its item is (rule 8), so the comment was
  nobody's.
- **At the file's end**: no statement is after it to be before.

## Decision

1. **A macro's opening on a line of its own, a path, `!` and `{`,
   `thread_local! {` or `jsx! {`, is a start**, as a closure's opening is
   (ADR 0581): a comment before it is its code's, a `thread_local!`'s
   first item's, a `jsx!`'s statement's. Its line, as an attribute's (ADR
   0606), is its code's, so a blank line before it is before its code and
   its comments: `// ATTENTION ..`, a blank line, `// The Symbol ..`; and
   a blank line before a component's `jsx! {` is before its `return (`.
2. **A comment after a module's last code, with only comments after it to
   where its code ends, ends its JS**, after its last statement, a blank
   line before it where the Rust has one. A module's code is its file,
   or an inline `mod`'s item up to its `}` (`module_code`): what's before
   an inline module's `}` ends that module's JS, not its parent's, whose
   code goes on past it.

## Why

- **Every comment of the Rust is in the JS** (ADR 0444): these were lost
  only for where they are.
- **A module says where its code is**, `LoweredModule::code`: the printer
  reads it, and doesn't work out from the text which module a `}` ends.

## Alternatives

- **An end comment as the last statement's trailing one**: it isn't on
  that statement's line, and oxc prints a program's comments at its end.
- **Only `thread_local!`'s opening**: a list of the macros whose body is
  code, which `jsx!` is too. Any macro's opening on its own line is one
  rule; an opening after code on its line, `let view = jsx! {`, is the
  statement's already.

## Consequences

- The guide's components whose Rust has a blank line before their `jsx!
  {` have one before their `return (`, and a `thread_local!` after a blank
  line has one before its `const`.

## What holds it

- `test/lowering.test.ts`, "a comment before a macro's opening is its
  code's, one at the end ends its module": a `thread_local!`, an inline
  module's end, and a file module's two-line end.
- Mutations `item-macro-comment-lost`, `item-macro-blank-lost`,
  `end-comment-lost`, `end-comment-past-code`,
  `inline-module-code-its-files`.
