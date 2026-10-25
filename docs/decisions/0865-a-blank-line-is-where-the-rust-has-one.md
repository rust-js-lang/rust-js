# 0865. A blank line is where the Rust has one between the code of the JS

Status: Accepted. Supersedes [0303](0303-blank-lines.md) where the source
tells, and keeps its rule where it can't. Amends
[0444](0444-a-comment-is-where-its-code-is.md)'s fifth rule, whose blank
lines between top-level statements were rust-js's, and its "part" of an
object, which had none.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): blank lines, which
nothing runs.

## Context

A line-by-line review of React's ports against upstream found blank lines
that follow no source:

- **Added between top-level statements.** ADR 0303 set apart two whose
  either was a function or spanned lines. React's `ReactBaseClasses` writes
  `const emptyObject = {};` and the `if (__DEV__) { Object.freeze(..) }`
  right below it as one block, and its port writes the `static` and the
  `js::on_load!` right below it so; its JS had a blank line between. So
  `ReactSharedInternalsClient` after its literal, `ReactChildren` before
  `function escapeUserProvidedKey`.
- **Dropped inside an object literal.** `ReactNoopUpdateQueue`'s object has
  a blank line between its methods, as its port's struct literal has
  between its fields; a field's comments were kept (ADR 0708), the blank
  line before them wasn't.
- **Dropped before a statement after a `let` with no JS.** `ReactChildren`
  sets `const iterator = ..` apart from the `if (__DEV__)` before it; its
  port's erased cast is between them, `let iterator_fn: &dyn Fn() -> .. =
  unsafe { js::cast(..) };`, which is read in place, and the blank line
  before it was before nothing printed.

## Decision

**A blank line is in the JS where the Rust has one between the code its
lines are of, and nowhere else.** The source's spans tell:

1. **Between two top-level statements**, the code of the first ends where
   the last of its nodes is in the Rust, and the second's starts where it
   does, or its first comment (`to_oxc::apart`). Where both are in one file,
   the first ending on a line before the second starts, a blank line in
   the Rust between them is one in the JS: a type with no JS between, an
   `impl`'s opening, `js::on_load! {`, is between too. Where there's none,
   there's none: two functions written right below each other in the Rust
   are in the JS.
2. **Where the Rust can't tell**, a statement rust-js makes of no code of
   its own, the corpus's `entry`, of another's, a trait's dictionary
   printed before what's written above it, or several made of one line, a
   `#[derive(Debug, Serialize)]`'s functions, ADR 0303's rule sets them
   apart as a person does (`format::spaced`, told which the source told
   of by `Written::told`).
3. **Between an object literal's fields** a blank line is where the struct
   literal has one, as Prettier keeps one between properties; none before
   the first, and none between a call's arguments, which Prettier keeps
   none between.
4. **A `let` with no JS**, an erased cast's, is no code between a blank
   line and the statement after it (`unprinted_let`), as it's none between
   a comment and it (ADR 0603); one that has JS, a statement starts in, is.

## Why

- **The JS reads as the Rust is laid out** (AGENTS.md): where a port's
  Rust is upstream's layout, its JS is too, and the source decides, not a
  rule of rust-js's.
- **One rule, at every level**: inside a function, a blank line already
  followed the source (ADR 0444); a module's top level and an object now
  do too.

## Alternatives

- **ADR 0303's rule everywhere, as before**: a blank line where the Rust
  has none, `const emptyObject = {};` apart from its `if`.
- **No blank line where the Rust can't tell**: the corpus's generated
  `entry` and a derive's functions on top of each other, which no person
  writes.

## Consequences

- A port lays its blank lines out as upstream's: `ReactBaseClasses`'s
  `function ComponentDummy() {}` and `ComponentDummy.prototype = ..` are
  one block where its Rust has no blank line between the `impl` and the
  `js::on_load!`.
- 259 corpus cases, 28 snapshots and the guide's examples lose a blank line
  between functions and items written right below each other, and a few
  gain one where the Rust has one: between enums' constants, before a
  statement after a `let` with no JS. A trait's dictionary is right above
  its first method, as the `impl`'s opening is above it.
- `docs/readability.txt` is blessed for its fewer lines, its counts the
  same.

## What holds it

`test/lowering-comments.test.ts`, "a blank line is where the Rust has one
between the code its JS is of", run; mutations `top-level-blank-untold`,
`top-level-gap-blank-unwritten`, `one-line-told`, `erased-let-blank-lost`
and `field-blank-lost` (`to_oxc.ts`) and `told-statements-spaced`
(`format.ts`).
