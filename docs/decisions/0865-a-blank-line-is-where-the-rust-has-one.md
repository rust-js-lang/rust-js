# 0865. A blank line is where the Rust has one between the code of the JS

Status: Accepted. Supersedes [0303](0303-blank-lines.md) where the source
tells, and keeps its rule where it can't. Amends
[0444](0444-a-comment-is-where-its-code-is.md)'s fifth rule, whose blank
lines between top-level statements were rust-js's, and its "part" of an
object, which had none. Amended: an `if let` chain made of a labeled
block is where its first `if` is. Amended (2026-10-25): a guard's `let` too, whose
`try` starts where its first statement does, and what's after the code,
an `export default`, tells nothing of it. Amended by [0883](0883-a-closure-of-its-calls-value-is-an-arrow-of-it.md): before an `if` whose test has comments after its keyword, the blank line is looked for before the `if`. Amended (2026-10-26): what's before a line is before the first JS printed of what's written on it.

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

## Amendment: an `if let` chain made of a labeled block is where its first `if` is

React's `ReactForwardRef` sets its `if (render != null && render.$$typeof
=== REACT_MEMO_TYPE) { .. } else if (..) { .. }` chain apart from the `if
(render != null) {` below it, as its port's `if let` chains are; the JS
had the blank line before the first chain instead, where the Rust has
none. A test MIR reads through a temporary, the `$$typeof` it compares,
makes the first chain a labeled block of MIR's, `l: { if (a) { if (b) {
s; break l; } } t }`, of no Rust of its own, which tidy makes `if (a && b)
{ s } else { t }` (ADR 0683). That `if` kept the block's span, none, so it
was printed at the start of the next statement that has one (ADR 0444):
the second chain's, after the blank line.

**The `if` made of a labeled block is where the program wrote the block,
`'desc: {`; of one MIR made, where its first `if` is** (`made_where`,
`tidy/logicals.rs`), as every other `if` is: its source map entry is its
first `if let`'s pattern, as the second's is its own.

Held by `test/lowering-comments.test.ts`, "an if let chain made of a
labeled block is where its first if is", run, and the mutation
`labeled-chain-unplaced` (`tidy/branches.ts`).

## Amended (2026-10-25): a guard's `let`, and what's after the code

Case ([0262](0262-when-rust-and-js-disagree.md)): N, blank lines alone.

React's ports had a blank line lost and one added:

1. **Before a guard's `let`.** startTransition sets `try {` apart from
   `ReactSharedInternals.T = currentTransition;`, and its port the guard
   whose drop is the `finally`, `let _finish = FinishTransition { .. };`,
   which prints nothing. The `try` a run a value's drop ends is made of
   (ADR 0437) was placed where the drop is, the scope's end, and the
   blank line was before its first statement, inside its block, where
   none is printed. **The `try` starts where its first statement does**,
   of a call's own `try` its first statement's (`scoped_drops`,
   `mir/tidy/unwinds.rs`): as rule 4 has it, a `let`
   with no JS is no code between a blank line and the statement after it,
   which is the `try`. What's before its first statement is before the
   `try`, a comment too. The scheduler's `unstable_runWithPriority`,
   `unstable_next` and `unstable_wrapCallback` and shared's
   `describeNativeComponentFrame` set their `try` apart as upstream does.
2. **After a literal, in ReactSharedInternalsClient.** Its `const
   ReactSharedInternals = {..};` and the `if (enableGestureTransition)`
   of its `js::on_load!` are right below each other, as the Rust's
   `thread_local!` and the macro are, but the JS set them apart. What the
   Rust tells of the code's statements, `Written::told`, was read of the
   module's last statements, and its `export default`, written after the
   code, was one more: each was read of the statement before, and the
   literal's, which nothing is before, said nothing, so ADR 0303's rule
   set the `if` apart. **What's after the code, an `export default` and
   a `pub use`'s `export { .. }`, says it's told**, written apart already
   (`to_oxc::emit`), so the rest is of the statement it's of.

Held by `test/lowering-comments.test.ts`, "a blank line before a guard's
let is before its try, and a default export tells nothing", run; and the
mutations `guard-try-at-drop`, `guard-try-first-unwritten` and
`guard-try-past-first` (`mir/tidy/unwinds.ts`; the corpus's
`temporaries_taken_apart` sets its `try` apart where the
Rust's `let` is), `default-export-told-untold` and
`aliases-told-untold` (`to_oxc.ts`).

## Amended (2026-10-26): before the first JS printed of what's written on the line

Case ([0262](0262-when-rust-and-js-disagree.md)): N, blank lines and
comments alone.

React's markRootFinished sets `const hiddenUpdatesForLane =
hiddenUpdates[index];` apart from the lanes it clears above it, as its
port sets apart `if let Some(hidden_updates_for_lane) =
hidden_updates[index].take() {`. The `take()`'s statements, `const old =
slots[index]; slots[index] = undefined;`, are printed before the `if`,
but written after its pattern, where the `if`'s span is: what's before
the Rust statement was before the first JS printed of its own span
(`printed_first`), the `if`, so the blank line was between the
`take()`'s statements and the `if`, and so was a comment above the line.

**What's before a Rust statement is before the first JS printed of what's
written from where it starts to its line's end**, its span's or not:
before `$index(hiddenUpdates, index)`, `const old = ..`, as a person
writes the take first. A line's code printed after what's below it isn't
reached: that's of another start. The corpus's `catch_unwind_chained`,
`chain_sides_own_steps`, whose comment was in a drop's callback,
`let_chain_values`, `mut_ref_temporary`, `or_bindings`,
`refcell_quiet_borrows`, `slice_disjoint`, `slice_views`,
`thread_local_set_before_use` and `untagged_enums`, and the playground's
`result_frame`, have the blank line or the comment before what their
line's Rust makes first, `const shape = $index(shapes, 4);` after `// In
an if let`, a static's `if let`'s `const limit = ..` after its comment;
`thread_local_set_before_use`'s `HELD.set(..)` gains the blank line it
had before nothing printed first.

Held by `test/lowering-comments.test.ts`, "a blank line before an if let
is before what its test's take() prints first", run, and the mutation
`printed-first-of-its-span-alone` (`to_oxc.ts`).
