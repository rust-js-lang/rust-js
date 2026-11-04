# 0444. A comment is in the JS where its code is, and so is a blank line

Status: Accepted. Amended (2026-11-04): a comment above a later arm of a tuple's pattern is that arm's, first in its `else if`'s block or above its guard `if` (#617). Amended (2026-11-04): a comment above a later or-pattern arm keeps its `else if` (#608); one above a later arm of a returned `match` is above that arm's `if` (#609). Amended (2026-11-04): a comment in or above a later match arm keeps its `else if` (#595). Amended (2026-11-03): a comment after a drop guard's `let` is after its `finally`'s statement. Amended (2026-11-03): an index loop's item is before its body's comments. Amended (2026-11-01): a `// SAFETY:` comment is left out of the JS, the user's decision. Amended (2026-10-29): a comment above a `use` is above the import it makes; an inner doc comment is JS's plain one. Amended (2026-10-28): a field's doc comment is its struct's one literal's. Amended (2026-10-26): a blank line before a statement rustfmt starts with a receiver on a line before its call is kept; a comment before a `match` an `if` is made of is the `if`'s. Extended by [0835](0835-an-object-is-across-lines-where-its-literal-is.md):
an object is across lines where its struct literal is. Extends [0383](0383-a-files-leading-comments-are-its-jss.md),
whose rules for what JS can hold are now every comment's, and
[0018](0018-print-with-oxc.md). Its amendment, a branch's comment keeps it a statement, is
superseded by [0658](0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md):
the comment is before the branch's value in the conditional. Amended by
[0733](0733-a-line-comment-ends-the-line-of-its-code.md): a line comment
after code a later line's JS follows ends the line of its code. Extended by
[0708](0708-a-comment-before-an-argument-is-before-it.md): a call's
argument and an object literal's field are starts too.
Extended by [0753](0753-a-branch-of-only-comments-keeps-them.md): a block
of only comments keeps them, where it ends; an `else` of only comments is
one.
Amended by [0865](0865-a-blank-line-is-where-the-rust-has-one.md): a
blank line between top-level statements, an object's fields, and after a
`let` with no JS, is where the Rust has one, and nowhere else (rule 5).
Amended: an `unsafe` block's opening is no code between a comment and
what it's before, and a `None` field a comment is written on keeps its key
([0280](0280-undefined-fields-left-out.md), amended). Amended
(2026-10-25): a comment before a `let` read in place is the next
statement's in its block, one in a `catch_unwind` closure is in the `try`,
and one before a binary's right operand is after its operator. Amended
(2026-10-25, again): an assignment that drops what its place held is
where its Rust is, and an or-pattern's alternative and a let-chain's `&&
let` are operands too. Amended (2026-10-25, a third time): an `if` a
match's tree makes of MIR's labeled block is where its first `if` is.
Amended (2026-10-25, a fourth time): an `if (a || b)` of guards is where
its first guard is, and a comment between `if` and its test is after
its `if (`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): comments and blank
lines, which nothing runs.

## Context

A file's leading comments, its license, were its JS's (ADR 0383); every
other comment was gone. A function's `///` docs, the `// Why b is two.`
before a line, the `// stdin` after an argument: the JS of a port read as
its original did, but without the comments that said why, and without the
blank lines that split a function into its steps. A person reading or
debugging the JS, or a JS caller reading a function's docs in an editor,
had only the code.

## Decision

**Every comment of the Rust is in the JS, before the statement whose code
follows it, as written; a `///` doc comment is a JSDoc. A blank line
between two statements in the Rust is one in the JS.** Decided with the
user: every comment, as written, and blank lines kept.

```rust
/// The sum, documented.
pub fn sum(x: u32) -> u32 {
    let first = x + a(); // Its length.

    // Then b.
    first + b()
}
```

```js
/** The sum, documented. */
export function sum(x) {
  const first = (x + a()) >>> 0; // Its length.

  // Then b.
  return (first + b()) >>> 0;
}
```

1. **A comment on lines of its own is its next statement's**: the first
   statement or item the JS has that starts after it, in the same file, if
   no `;`, `{` or `}` of code is between them. One with code between, a
   comment inside an expression, isn't moved to a statement it isn't
   about. An item with no JS, a type's, takes its comments with it.
2. **A comment after code on its line stays at that line's end**: after
   the JS of the last line that code maps to.
3. **A `///` doc comment's lines are one JSDoc**, `/** one */` or
   `/**\n * a\n * b\n */`, which an editor shows a JS caller. A `/** */`
   is one already; `//!` is a plain comment, JS having no module docs.
   A `///` line with a `*/`, which would end the JSDoc, keeps the run as
   written, `///` lines.
4. **What JS can't hold as one comment isn't written**, the rule ADR 0383
   made for a file's leading ones, now one function, `comments::is_one_in_js`,
   asked by both: a block comment Rust nests, and a line comment with a
   bare carriage return, U+2028 or U+2029, whose rest JS would read as
   code. A CRLF's `\r` isn't the comment's.
5. **A blank line is kept** where the Rust has one between a statement and
   the one or the comments before it. Between top-level statements, rust-js
   still adds one where a person would (ADR 0295); a statement's comments
   right above it are counted as its.
6. **A temporary rust-js writes before a statement**, `const x = v[0];` of
   a `match`, comes after the statement's comments: they're printed before
   the first statement its code is printed as.
7. **An arrow whose one statement has a comment keeps its block**,
   `(x) => {\n  // Inside.\n  return x + 1;\n}`, where it'd be `(x) =>
   x + 1`, which has no line for it.
8. **A `thread_local!`'s `static` is where its name is**, not where the
   macro is called, so a doc comment of one is its `const`'s.

## Why

- **Generated code is a product**: the comments are what a person wrote
  for the next reader, and the JS is what's read in a debugger and a
  stack trace.
- **A port reads as its original** (AGENTS.md): react.dev's files have
  comments and blank lines, and their JS had neither.
- **JSDoc is JavaScript's docs**: a JS or TypeScript caller of a
  function sees its Rust docs in an editor, as a caller from Rust does.

## Alternatives

- **Only doc comments**: a function's docs for its callers, but not why a
  line is as it is, the comments a reader of the JS needs most.
- **Leading comments only, as before**: the JS a port writes loses what
  its original says.

## Consequences

- **A comment inside an expression the JS writes as one** isn't written
  yet: in a struct literal's field, between a call's arguments. Oxc's
  printer holds a comment before a statement, or at a line's end; no
  comment is moved where it doesn't belong. A branch's is kept (below).
- A `.d.ts` has an item's doc comments, not its other comments (below).
- The corpus, the examples' snapshots, the pilot's and the playground's
  committed JS have their comments and blank lines.

## Amendment: a `.d.ts` has its items' docs

**A declaration's doc comment is its JSDoc in the `.d.ts`**: a function's,
a type's, a `const`'s, and each field's before its property, as react.dev's
getRouteMeta documents `RouteItem`'s fields, which a TypeScript caller's
editor shows on hover. rustc's doc attributes give the lines, so a
`#[doc]` a macro writes is one too; one function, `comments::jsdoc`, makes
a JSDoc of doc lines for the JS and the `.d.ts`, none of lines with a
`*/`. @rust-js/typescript prints a declaration's `docs` before it, and a
property's before its line. A plain comment isn't a declaration's: it
documents the code, which the `.d.ts` hasn't.

## Amendment: a branch's comment keeps it a statement

**An `if` or a `match` whose branch has a comment stays statements**,
`let n; if (flag) { // Yes.\n n = 1; } else { n = 2; }`, where it'd be
`const n = flag ? 1 : 2`, which has no line for the comment: as an
arrow's block is kept (7). Tidying folds no branch of a comment
(`branches::commented`), which it's told of by the body's comment spans,
`hints::Written::comments`, from where the `if` starts to where its last
branch's code ends.

## Amendment: across an `unsafe` block's opening

React's `resolveDispatcher` ends in its comment's code:

```rust
    // Will result in a null access error if accessed outside render phase. We
    // intentionally don't throw our own error because this is in a hot path.
    // Also helps ensure this is inlined.
    unsafe { dispatcher.unwrap_unchecked() }
```

The `{` of `unsafe {` was code between the comment and the `return` its
body is, so the comment was nobody's and lost; written inside the block,
it was kept. **An `unsafe` block's opening is a start, as a closure's
`|queue| {` is (ADR 0581)**: the block has no JS but its code's, so a
comment before it is before that code, `// Will result .. return
dispatcher;`. The corpus's `// SAFETY:` comments before an `unsafe` block,
lost before, were kept too, until the amendment below left them out.

Held by `test/lowering-comments.test.ts`, "a comment before an unsafe block
is its code's", and the mutation `unsafe-opening-code` (`to_oxc.ts`).

## Amended (2026-10-25): a run before a match arm is one, and none is lost

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

React's `startTransition` writes five lines before what a nested
Transition shares; its port writes them before the `match` arm, `Some(prev_transition)
=> prev_transition.types.borrow().clone()`. The last line was the arm's
value's, in the `if`, and the four before it before the `if`: what was
between a comment and the arm was taken for code where it was the run's
other comments, so only the last, right above the arm, was the arm's
(ADR 0590). And a `match` read as `prev?.types`, a statement of no arm,
lost them all: they were before a start nothing printed.

**A run of comments is one where what's between them is only comments**,
so each line before a match arm is the arm's value's where the last is;
and **a comment before nothing printed is before the innermost statement
it's in**, not lost, a module's item aside, a function's, far above
where it's written: `// If we're a nested transition .. current.types =
prev?.types;`.

Held by `test/lowering-comments.test.ts`, "a match arm's comment of many
lines is its value's, none lost", and the mutations
`arm-comment-run-split` and `unattached-comments-lost` (`to_oxc.ts`).

Then an `Option` never falsy was tested by its truth (ADR 0298, amended),
and that `match` was `prev ? prev.types : undefined`, no `?.`: the arm
was a conditional's branch again, whose comments the printer writes after
its `?`, where the map places its first JS (ADR 0658). None of it was
placed in the arm: oxc places a member by its object, and `prev`, a name
read where it's bound, was placed at `Some(prev)`; `String(n)`, of
`n.to_string()`, nowhere. The comments were lost again. **A branch with
comments has its first JS placed where its value starts** (`Cx::branch`,
`to_oxc/expressions.rs`), and `String(n)` where it's written, so they're
before the value in the conditional, as React's `prevTransition !== null
? // If we're a nested transition .. prevTransition.types : null` is; an
arm read away, `n ?? given`, keeps them before its statement. Held by the
same test, and the mutations `branch-first-unplaced` and
`stringed-unplaced` (`to_oxc/expressions.ts`).

## Amended (2026-10-25): past a `let` read in place, in a caught closure, between operands

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

React's port had three comments drift and one stay where it's written:

1. **Before a `let` read in place.** `// Objects go into a WeakMap` is
   written before `let arg: &JsObject = unsafe { js::cast(arg) };`, which
   has no JS, `arg` read where it's named, then `let object_cache = match
   cache_node.o.get() { .. }`. A `let` with no JS was code between a comment
   and the next start only where it had no `{` (ADR 0603); and that next
   start, `let objectCache = cacheNode.o`'s, is where it matches, past `let
   object_cache = match`. The comment was nobody's, and before the
   innermost statement it's in, the `for`, outside its block. **What's
   between a comment and a start may be whole `let`s, each to its `;`
   outside its brackets, none closing its block's, then what the start's
   own code begins with**: the comment is the next statement's, in its
   block, as React's is. `leading_statements` (`to_oxc.rs`) says which are
   whole; a bracket or `;` in a string only ever makes fewer whole, so the
   comment falls back as before, never further.
2. **In the closure `catch_unwind` calls.** `// $FlowFixMe[..]` is written
   in `catch_unwind(AssertUnwindSafe(|| { // .. \n function::apply(..) }))`,
   whose call is the `try`'s, `const result = fn.apply(null, args)`. That
   statement was placed where the `match` is, so the comment, inside it,
   was before the `try`. **What's made of the caught call, its `const`,
   its assignment or its statement, is placed where the call is**
   (`try_catch`, `prepare/catches.rs`), and the comment is in the `try`,
   as upstream's. A comment before the `match` is still before the `try`.
3. **Between a binary's operands.** `escapedPrefix + // $FlowFixMe[..]
   (mappedChild.key != null && .. ? .. : '') + childKey`: the port writes
   the comment before the `+`, as rustfmt puts an operator first. Its
   operand was no start, so it was before the statement, `const newChild`,
   where it read as a copy of the line above; and `a // c\n && b` lost its
   comment. oxc's codegen prints a leading comment before a binary's right
   operand only where it's an annotation, `/* @__PURE__ */`
   (`print_annotation_comments_before_expression`: a moved statement's
   comments would land in an operand), and a parenthesized expression's
   only as a property's value, a conditional's alternate or a template's
   expression. So **a binary's right operand is a start, as a branch is
   (ADR 0658), its comments written after its operator**, where the map
   places its first JS (`Cx::branch`), on lines of their own, which oxc's
   formatter lays out as Prettier does: `escapedPrefix +\n // $FlowFixMe
   ..\n (mappedChild.key ..`. Each start says its operator, `?`, `:`, `+`
   or `&&`, which the split is after (`arm_starts`). An operand takes a
   comment only where nothing but operators and openings, `+ &(`, is
   between them: `None => given` of a match read as `n ?? given` is code,
   and its arm's comment stays the statement's. One in a closure read as
   the operand, `x.is_some_and(|v| { // c\n v > 3 })`, is after its `&&`.
   A `/* */` of the comment in the line was the other way: it changes what
   was written.
4. **Before `children?;`.** `// $FlowFixMe[..]` is before the `if
   (children == null) { return null; }` its statement is, where the Rust
   has it; upstream has it inside, before `return children`. Clippy's
   `question_mark` asks for `children?;` of `if children.is_none() { return
   None; }`, a comment inside it or not, so the port writes the comment
   before that, and the JS keeps it there: no change.

Held by `test/lowering-comments.test.ts`, "a comment before a let read in
place is the next statement's, in its block", "a comment in
catch_unwind's closure is in the try", "a comment between a binary's
operands is before its right one" and "a comment before a ? statement is
before its if"; and the mutations `erased-let-block-code`,
`erased-lets-before-nothing-else`, `operand-no-start`, `operand-past-code`
and `operand-after-any-operator` (`to_oxc.ts`), `operand-first-unplaced`
(`to_oxc/expressions.ts`) and `caught-call-at-match`
(`prepare/catches.ts`). The corpus's `plain_data` keeps a comment it lost,
before a `transmute` read in place.

## Amended (2026-10-25): in an assignment that drops, before an alternative or a let-chain's `let`

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

A line-by-line review of React's ports found three comments lost or moved:

1. **In an assignment over a value it drops.** startTransition's port
   writes `*current_transition.types.get_mut() = match prev_transition {
   // If we're a nested transition .. \n Some(prev) =>
   prev.types.borrow().clone(), None => None };`. What the place held, an
   `Option<Rc<..>>`, has a drop, so MIR drops it and writes the new one,
   each where the place is, `*current_transition.types.get_mut()`: the
   statement, `currentTransition.types = prevTransition?.types ?? null`,
   was placed there alone, and the arm's comments, read away into `?.`,
   were in no statement, and lost. **An assignment is where its Rust is,
   from its place to the end of the value it's given**
   (`FnCx::assignment_span`, `mir/assign.rs`): where MIR's span is the
   place's alone, the value a temporary written right after its `=`, to
   the temporary's end. The comments are before the statement, as the
   rule above has them.
2. **Before an or-pattern's alternative.** React's cache writes `typeof
   arg === 'function' || // $FlowFixMe[invalid-compare] \n (typeof arg ===
   'object' && arg !== null)`; its port, `if let Kind::Function(_) //
   $FlowFixMe[..] \n | Kind::Object(_) | Kind::Array(_) =
   js::classify(arg)`. A pattern's `|` is no binary of rustc's tree, and
   the JS's `||` was placed nowhere, so the comment was before the `if`.
   **Each test of an or-pattern is where the first alternative it tests
   is written** (`written` of `variants_test` and `untagged_test`, of
   `FnCx::written_variants`), `Object | Array`'s one test where `Object`
   is, so a comment before an alternative is before its operand, after
   its `||`, as a binary's right operand's is. An `if let`'s pattern is
   read as a `match` arm's is, and a reference's, match ergonomics' `&`,
   so their alternatives are tested as written too (ADR 0780, amended).
3. **Before a let-chain's `&& let`.** startTransition's `js::type_of(v) ==
   "object" // $FlowFixMe[..] \n && let Some(v) = v`: the JS's operand,
   `v != null`, is placed at the pattern, past `let`, which was code
   between, so the comment was lost in one and before the inner `try` in
   the other. **A let-chain's `let` is an operand's opening**, as `(` and
   `&` are: what's between a comment and its operand may be `&& let`.

Held by `test/lowering-comments.test.ts`, "a comment in a value assigned
over one dropped is before its statement" and "a comment before an
or-pattern's alternative or a let-chain's let is before its operand", run;
and the mutations `dropping-assignment-at-place` (`mir/assign.ts`),
`alternative-unplaced` (`untagged.ts`), `variant-alternative-unplaced`
(`representation.ts`), `if-let-alternatives-unwritten` and
`deref-alternatives-unwritten` (`mir/temporaries.ts`) and
`let-chain-let-code` (`to_oxc.ts`).

## Amended (2026-10-25): an `if` a tree of tests makes is where its first `if` is

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

React's describeNativeComponentFrame writes `// We couldn't find our
intentionally injected common root frame ..` before `if (s ===
sampleLines.length || c === controlLines.length) { .. while (..) { .. }
}`, and `// In V8, the first line ..` before `if (s !== 1 || c !== 1) {
do { .. } while (..); }`; its port, the same comments before `if s ==
sample_lines.len() || c == control_lines.len() {` and `if s != 1 || c !=
1 { loop { .. } }`. MIR's structure makes each a labeled block of no
Rust of its own, `l: { if (!a) { if (!b) { break l; } } s; while (..) {
.. break l; } }`, whose loop leaves it, so `decisions` makes the `if` of
its tree (ADR 0364), not tidy's chains of `&&` and `||`. That `if` was
placed where its first arm's first statement is, past the `{` the
comment is before: the comment was nobody's, and before the innermost
statement it's in (the amendment above), the function's `try` in the
one and the outer `while` in the other.

**The chain's first `if` is where the block is; of a block MIR made,
where its tree's first `if` is** (`made_where`, `tidy/logicals.rs`, the
one answer an `if` of a labeled block's chain is placed by, ADR 0865's
amendment), so the comment is before it, as upstream has it, and its
source map entry is its first test. An `else if` of a later arm stays
where its arm's code is: a comment before an arm is its value's (above),
never between `else` and `if`. ReactJSXElement's `// Resolve default
props` and the comment before the corpus's `or_bindings`' `if let` were lost
the same way, and are kept, the latter after the `const` of what its
`if let` tests, which is placed at the scrutinee, past the pattern's
braces.

Held by `test/lowering-comments.test.ts`, "an if a decision tree makes
is where its first if is", run, its `if`s mapped to their tests; and the
mutation `decision-chain-at-first-arm` (`mir/tidy/decisions.ts`).

## Amended (2026-10-25): an `||` of guards, and inside an `if (`

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

React's ReactTaint writes `// $FlowFixMe[invalid-compare]` inside `if (`,
before `lifetime === null ||`; its port, before or inside `if
matches!(js::classify(lifetime), Kind::Null) || (..) {`. Two were lost:

1. **Before an `if (a || b)` whose branch ends**, a `panic!`'s. MIR's
   structure makes it a labeled block of guards, `l: { if (a) { break l; }
   if (b) { break l; } rest } throw`, which `guarded` (`tidy/jumps.rs`)
   makes `if (a || b) { throw } rest`, placed where the block is: nowhere,
   as MIR made it. The comment before it was before the next `if`, and
   that one's own lost. **It's where its first guard is** (`made_where`,
   the answer the `if`s of `&&` and `||` chains are placed by), its test.
2. **Between `if` and its test**, which rustfmt leaves where it's written.
   The `if` is placed at its test, so the comment was before the `if`,
   or, of an `if` placed nowhere, lost. **An `if`'s test is a start whose
   comments are those written after the `if`, the keyword, past other
   comments** (`after_if`, `to_oxc.rs`), written after its `if (` as a
   binary's right operand's are after its operator, on lines of their own,
   which the formatter lays out as Prettier does: `if (\n // $FlowFixMe
   ..\n lifetime === null ||\n (..)\n ) {`, as upstream has it. A comment
   before the `if` stays the statement's; the test is placed where its
   first part placed is (`written_extent`), an `||` tidy made being placed
   nowhere itself.

Held by `test/lowering-comments.test.ts`, "a comment before an or's if or
inside it before its first operand is there", run; and the mutations
`guarded-or-at-block` (`mir/tidy/jumps.ts`), `if-test-no-start`,
`if-test-takes-any-comment` and `if-opening-unsplit` (`to_oxc.ts`).

## Amended (2026-10-26): in an `else`'s block, before an `else if`, past an operand

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

React's ReactFiberLane, and react.dev's llms.txt, lost three comments:

1. **One an `else`'s block starts with, before its only `if`**, `} else {
   // Nothing has been pinged. ..\n if (!rootHasPendingCommit) {`. A lone
   `if` in an `else` printed as `else if`, where oxc writes no comment
   before it. **Where a comment of the Rust is before it, the `else`
   keeps its block, as it's written** (`Cx::commented`, the starts the
   comments are attached to, a blank line's not among them).
2. **One written before an `else if`'s `else`**, react.dev's `}\n// Single
   page without children - group under its own heading\nelse if (..)`.
   It was lost, and by (1) would be in a block. **It's before the `else`,
   after the `}`, on lines of their own, the `else if` one still**, as
   Prettier keeps it and upstream has it: a branch's split (ADR 0658) after
   the `}` (`else_if_starts`).
3. **One before a parenthesized right operand**, `a >= b\n// Default
   priority ..\n|| (c && d)`, whose `&&` tidy made of branches is placed
   nowhere: no start, so the comment was lost. **Its start is where its
   parts are** (`written_extent`), as an `if`'s test's is.
4. **One after `if`, before a test MIR places at the keyword**, `if\n// Is
   this one of the newly entangled lanes?\n(lane & a) | (e[i] & a) != 0`:
   the rules a comment skips a start by, a call's part it's outside of,
   an operand with code between, an `if`'s test it isn't after, were each
   asked once, in turn, so one past an operand was then taken by a call's
   part, `$index(\n // ..\n entanglements, index)`. **Each start is asked
   every rule again**, so the comment is after `if (`, and the one between
   the operands before its operand, as upstream has them.

Held by `test/lowering-comments.test.ts`, "ReactFiberLane's comments in an
else, an if's test and a call are where it writes them", run;
`test/lowering-loops.test.ts`'s else-if tests; and the mutations
`commented-else-if` (`to_oxc/statements.ts`), `else-if-comment-in-block`,
`unplaced-operand-no-start` and `start-rules-asked-once` (`to_oxc.ts`).

## Amendment: a blank line before a method call on the line after its receiver

ReactFiberLane's `markRootFinished` separates `root.expiredLanes &=
remainingLanes;` and `root.entangledLanes &= remainingLanes;` by a blank
line. Its port's `root.expired_lanes\n    .set(..)`, as rustfmt writes a
`Cell`'s `set` too long for one line, gives a statement whose span starts
at `.set`, so the blank line was looked for before the `.set` line, the
receiver's, and none was found. **A statement whose span's line starts
with `.` starts on the line before it, its receiver's, for the blank line
before it**, as many lines back as the chain goes.

Test: `lowering-comments`'s "a blank line before a method call on the
line after its receiver is kept", run. Mutation
`blank-before-chain-at-its-call` (`to_oxc.ts`).

## Amendment: a `fn` a macro writes returns where its block ends

React's getComponentNameFromFiber is in `js::export_default! { fn .. }`,
its last value after a blank line, `}\n\n None`; its JS lost the blank
line, `}\n return null;`, which the same function written alone keeps.
MIR's `return` is where the `fn` ends, which in the macro is its
`macro_rules!`'s `$vis fn $($function)*`, so the statement was placed at
the whole invocation, no blank line before it. **A `fn` a macro wrote
around a block the program wrote returns where that block ends**, as one
written alone does (`structure.rs`'s `Return`): the blank line before it
is kept, and its map points there. Case N: where a statement is mapped,
which nothing runs. Held by `test/lowering-comments.test.ts`, "a blank
line before a function's last value in export_default! stays", and the
mutation `macro-fn-return-at-the-macro` (`lower/mir/structure.ts`).

## Amendment: a comment before a `match` an `if` is made of is the `if`'s (2026-10-26)

React's enqueueCapturedUpdate writes `// Append the captured update the
end of the cloned list.` before a `match &new_last { None => .., Some(last)
=> .. }`, which MIR makes an `if` of, mapped where its test is, the first
arm's pattern (`tidy.rs`'s `source_spans`). With nothing else printed of
the `match`, the code between the comment and that start, `match
&new_last {`, was code, so the comment was lost. **A `match`'s opening,
`match <scrutinee> {` on its line, right before its first arm, is no code
of its own**, as an `unsafe` block's and a closure's openings aren't
(`to_oxc/openings.rs`, which holds what opens such a block, out of
`to_oxc.rs` at its line cap): the comment is the `if`'s, `// Pick by
option, none first.` before `if (x == null) {`. Not where the start is past the
first arm's pattern, a statement in an arm, which the comment isn't
before. Case N: what's printed, which nothing runs.

Held by `test/lowering-comments.test.ts`, "a comment before a match an if
is made of is the if's", and the mutation `comment-before-a-match-lost`
(`to_oxc.ts`).

## Amendment: a comment before an item with no JS is before the next that has (2026-10-28)

React's ReactFiberConfigWithNoHydration heads a section, `// Hydration
(when unsupported)`, whose first lines are types, `export type
ActivityInstance = mixed;`, then its constants; Flow's JS keeps the heading
before `export const supportsHydration = false;`. A `pub type` writes no
JS, so what was between the comment and the next start was code, and the
comment was lost (React port gap rd12). **A type alias, its attributes and
visibility first, is passed over as a `let` with no JS is**
(`openings::type_alias`): the comment is before the next item that has JS.
`test/lowering-comments-items.test.ts`, "a comment before an item with no
JS is before the next item that has".

Amended (2026-10-28, React port gap rd58): **a doc comment of a type
alias is that item's, nowhere in the JS**, as an item's doc comment is
its own; only a plain comment heads the next item. The rule above made
each `/// What useMemo calls ..` before a `pub type` a stray `/** .. */`
before the next item, 41 in the port. `of_no_js` in `comments_of` leaves
out a doc comment whose code after it, past other comments, is a type
alias. "a doc comment of an item with no JS is nowhere in the JS";
mutation `doc-of-no-js-kept`.

## Amendment: a comment before a `catch_unwind` closure's value is in the `try` (2026-10-28)

React's isSameOriginFrame, `try { // Reading href can throw. return typeof
iframe.contentWindow.location.href === 'string'; } catch (err) { return
false; }`: its port's `catch_unwind(AssertUnwindSafe(|| { // ..
\n matches!(..) })).unwrap_or(false)` had the comment before the `try`
(React port gap rd28). Rule 2 above placed what's made of the caught call
where the call is, a `const`'s and an assignment's; the `Ok`'s value
returned as it is, `return f(x)`, kept the `match`'s place. It's the
call's now too (`try_catch`, `prepare/catches.rs`).
`test/lowering-comments-inside.test.ts`, "a comment before a
catch_unwind closure's value is in the try"; mutation
`kept-ok-at-the-match`.

## Amendment: before an operand that's an `if`'s value (2026-10-28)

React's beginWork, `oldProps !== newProps || hasLegacyContextChanged() ||
// Force a re-render if the implementation changed due to hot reload:
(__DEV__ ? workInProgress.type !== current.type : false)`: its port's
`|| (if __DEV__ { .. } else { false })` lost the comment (React port gap
bwc1), as only an operator and what opens its operand, `+ &(`, `&& let`,
were what a comment and the operand may have between them. **An `if`
opening it is one too**, `|| (if`: the comment is before the operand, the
conditional. `test/lowering-comments-inside.test.ts`, "a comment before
an operand that's an if's value is before it"; mutation
`operand-if-opening-code`.


## Amendment: a block doc comment's lines, and an empty one (2026-10-28, React port gap rd67)

A port's `/** .. */` doc comment, as upstream's JSDoc, kept its JS form,
but its declaration starred each line again, ` * * A block ..`, and kept
its first and last empty lines: rustc gives a block doc comment's text
as written, its stars too. **A declaration's JSDoc is of a block doc
comment's lines as rustdoc reads them** (`beautify_doc_string`: its
stars and its first and last empty lines left out), and of a line doc
comment's each line, an empty `///`'s too, which a paragraph break is
(`declarations/comments.rs`). Held by `test/declarations-comments.test.ts`,
"a block doc comment is its lines' JSDoc", and the mutations
`block-doc-starred-again` and `empty-doc-line-dropped`.

## Amendment: a field's doc comment is its struct's one literal's (2026-10-28, React port gap rd64e)

The user's decision. Case ([0262](0262-when-rust-and-js-disagree.md)): N,
where comments are written.

React DOM's SyntheticEvent documents its prototype object's fields:

```js
assign(SyntheticBaseEvent.prototype, {
  ..
  /**
   * Checks if this event should be released back into the pool.
   *
   * @return {boolean} True if this should not be released, false otherwise.
   */
  isPersistent: functionThatReturnsTrue,
});
```

The port's object is a struct's literal, `SyntheticEventPrototype { ..
}`, whose fields are documented where it's declared, `///` on each; the
struct has no JS, so the docs were printed nowhere but its `.d.ts`.

**A struct's field's doc comment is printed as JSDoc before that field
in the struct's literal, where the crate writes one literal of it**: the
one object the struct describes. The literal's own comments come after
it, and a blank line the literal has before the field before it.

- **One literal only** (`aggregates::struct_literals`, a count of each of
  the crate's structs' literals): a struct made in several places keeps
  its docs on its declaration, not a copy at each.
- **The crate's own structs**: std's `Range`, `a..b`'s literal, documents
  its fields for Rust's readers.
- **Carried as a fact of the object** (`ObjectLayout::docs`, each key's
  JSDoc, made where the literal is lowered), printed by `to_oxc` before the
  field's value, as a doc comment before an item is; a spread of the
  object written out keeps them (`prepare::objects`).
- **A `//` before a `let` stays `//`**, the user's decision too: rustc
  warns of a `///` there (`unused_doc_comments`), and a comment's kind
  doesn't change by what follows it. Upstream's `/** .. */` before its
  `function SyntheticBaseEvent` is `//` lines in the port's JS.

The playground's compiler's snapshot gains its structs' field docs where
each is made once, `/** WASI's own name for its imports. */`.

Held by `test/lowering-comment-fields.test.ts`'s "a field's doc comment is
before it in its struct's one literal" (a struct of one literal, one of
two, a blank line before a field, a `//` before a `let`), and
`test/lowering-structs-enums.test.ts`'s "across lines where" (a range's
`start` and `end` undocumented); mutations `field-docs-of-any-literal`,
`field-docs-of-std`, `field-docs-left-out` (`lower/aggregates.ts`) and
`field-doc-after-blank-line` (`to_oxc.ts`); the guide's `prototype`.

## Amendment: a comment above a `use` is above its import (2026-10-29)

React's ReactDOMInput heads an import with a TODO, `// TODO: direct
imports like some-package/src/* are bad. Fix me.`, above `import
{getCurrentFiberOwnerNameInDevOrNull} from
'react-reconciler/src/ReactCurrentFiber';`. Its port's comment, between
two `use`s, was lost (the port's gap rd92): the rule above takes a comment
to the next statement only across code with no JS of its own, and a `use`
was none of it; and an import, plain text above the code (ADR 0295), had
no place in the Rust a comment could be before.

**Each import is where the `use` that makes it is written**, the first of
the module's `use`s naming what it imports: a library's item, a binding of
its JS file, or of a module of the crate's, its item or the module or one
it's in (`analysis/naming.rs`'s `UsesWritten`, `js::Import::span` and
`js::Package::span`). A `use` tree's leaves are where its statement is. A
comment above a `use` is above the import it makes, in the header,
wherever the imports' order puts it, and once, of two imports one `use`
makes, above the first printed (`to_oxc/header.rs`). A `use` that makes
no import, a type's, is code with no JS, as a type alias is: a comment
above it is above the next import, or with none the module's first
statement, as written, the blank line after the `use`s too. The
coordinator chose this of two designs, with every comment among the
`use`s above the imports, in order, the other, which isn't where upstream
writes its TODO.

So a module's `//!` before its `use`s, a plain comment (rule 3 above),
is above its first import, or its code, as one before its first item was:
the examples' `builtins`, `counter`, `todomvc`'s `item` and `web_forms`
now keep theirs.

- Held by `test/lowering-comments-items.test.ts`, "a comment above a use
  is above the import it makes", run: of a crate module's import, a
  binding's JS file's, a `use` tree's, a module's `use`, and a type's
  `use` before an import and before the first statement. Mutations
  `use-comment-in-the-code`
  (`to_oxc.ts`), `use-comment-twice` (`to_oxc/header.ts`),
  `type-use-is-code` (`to_oxc/openings.ts`), `use-tree-at-its-leaf`,
  `binding-use-makes-no-import` and `module-use-makes-no-import`
  (`lower/analysis/naming.ts`).

*Amended (2026-11-04, #642):* `comments_of` no longer asks
`commented_at` first where a comment at a `use` goes. Its mutation,
`use-comment-dropped`, survived the nightly (#606): since da526f90e a
forwarder's comments are in `js::Module::taken_into`, whose map already
goes through `commented_at`, and an import's comment reaches the
statement printed first of its `use`, which `commented_at`'s keys make a
start, so both arms gave one answer. At 14b2d5053 the mutant still lost
the test's forwarder doc comment. With it, every corpus case's JS was the
same (1,602 cases), and twelve comment, module, re-export and snapshot
suites passed (210 tests). The arm is removed with its mutation, so one
place answers the question: `commented_at` through `taken_into`, and the
start it adds.

## Amendment: an inner doc comment is written as JS's plain one (2026-10-29)

The coordinator's decision. Case ([0262](0262-when-rust-and-js-disagree.md)):
N, no observable difference: a comment.

Rule 3 made `//!` a plain comment, but it was printed as Rust writes it,
`//! Where the app is: ..`, above the pilot's imports, which no JS file
writes: JS has no doc comment of the module or block it's in, and a
`/*! */` is a minifier's "keep this", a license's.

**An inner doc comment, `//!` or `/*!`, is written as JS's plain comment,
`// Where the app is: ..` and `/* .. */`**, wherever it's printed: above
the imports (`to_oxc/header.rs`), before a statement (oxc, which prints a
comment's text from its span, so its JS form is written past the source,
as a case's `// falls through` is), and after code or a value. One
function says it, `comments::in_js`. The pilot's committed JS and the
`builtins`, `todomvc` `item` and `web_forms` snapshots are re-blessed.

- Held by `test/lowering-comments-items.test.ts`, "a module's inner doc
  comment is a plain JS comment", run: a module's `//!` lines above its
  import, and an inline module's `/*!` above its code. Mutations
  `inner-doc-kept-in-js` (`comments.ts`) and
  `inner-doc-printed-from-source` (`to_oxc.ts`).

## Amended (2026-11-01): a `// SAFETY:` comment is left out

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

Clippy's `undocumented_unsafe_blocks` asks for a `// SAFETY:` comment
above each `unsafe` block, and the port writes them, `// SAFETY:
mount_transition stored an Rc<dyn Fn(bool)> there.` above `unsafe {
js::cast(..) }`. Kept, it was in the JS above `return hook.memoizedState;`:
it speaks of an `unsafe` the JS has none of, of Rust's ownership and
validity, and upstream's file has no such line, so the port's JS read
unlike its original.

**A comment that begins `SAFETY:` is left out of the JS and the `.d.ts`**,
the user's decision, and so are the line comments on the lines right below
one on lines of its own, the rest of what it says; a doc comment, `///
SAFETY:`, is its item's, and one that begins otherwise, `// Safety:`, is
kept. One function says which, `comments::left_out`; the source arena
keeps each such comment, not `written`, so what's between code and a
comment is still no code; the printer places it and writes nothing, and
the blank line above it is the next code's. Where lowering asks whether
a comment is somewhere, a `None` given at a function's end, a jump's, a
block that ends with one, a `// SAFETY:` one gives the same JS as none:
those keep no statement of their own that prints.

Alternatives: keeping it, as before, which the port's JS showed upstream
didn't have; leaving out every comment above an `unsafe` block, which
loses what a comment says of the code itself, React's resolveDispatcher's
`// Will result in a null access error ..`.

Held by `test/lowering-comments-left-out.test.ts`, "a SAFETY comment is
left out of the JS, the comments around it kept", `comments.rs`'s unit
test, and the mutations `safety-comment-written`, `safety-run-unended`,
`safety-run-after-code` (`comments.rs`), `safety-arena-written`
(`sources.rs`), `safety-blank-lost`, `safety-statement-comment-written`,
`safety-line-end-written` (`to_oxc.rs`) and `safety-declaration-written`
(`declarations/comments.rs`).

## Amendment: a `static`'s array's items are where they're written (rd344a, 2026-11-02)

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments.

React's ReactFlightPerformanceTrack writes `const trackNames = ['Primary',
'Parallel', 'Parallel\u200b', // Padded with zero-width space to give
each track a unique name.` The port's `static TRACK_NAMES: [&str; 10]`
printed no comment: a `static`'s or a `const`'s value is rustc's value of
it, whose items are where nothing is written, so no JS of the comment's
line was there to be after. **Each item of such an array is where its
initializer writes it** (`literals::const_placed`, which places a
number's literal too, ADR 0627), and a line comment after one is after its JS. Held by
`lowering-comments-items.test.ts`'s "a comment after an item of a
static's array is after that item", run, and the mutation
`static-array-items-unplaced` (`literals.ts`).

An array a function's body borrows, `f(1, &[""], // path`, rustc's
promoted constant, printed the comment after the call's `)` (since: see the rd525 amendment below).

## Amendment: a comment after an array's statement is after the statement (rd605, 2026-11-02)

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments.

React DOM's BeforeInputEventPlugin writes `const END_KEYCODES = [9, 13,
27, 32]; // Tab, Return, Esc, Space`. Once rd344a placed each item where
it's written, the port's `const END_KEYCODES: [f64; 4] = [9.0, 13.0,
27.0, 32.0]; // ..` printed `32, // Tab, ..` inside the array, laid out an
item a line: the JS before it's laid out writes an array of more than two
items an item a line, and the last JS of the comment's line is the last
item's line, before the `];` no code of the line maps to. **The JS's
lines after it that hold only brackets the JS of the comment's line
opened and left open are its line's too**, as many as the Rust's line
ends closing (`trailing::closed_after`, by `Trailing::closes`, the `]` of
`32.0];`), so the comment is after the statement, `[9, 13, 27, 32]; //
Tab, ..`, as upstream, and after a block's last array, `return [9, 9,
9]; // ..`, not after its `}`. A bracket the Rust closes that the JS has
none of, `o: Cell::new(None), // object cache`'s `)`, is no reason to go
past the object's `};`: the JS of that line, `o: null,`, opens nothing. A
comment after an item, `"Parallel", // ..` before a line of the array's
rest, closes nothing and stays after it. An array that rustfmt wouldn't
write, its last item and `];` on a line after its first, `1, 2,` then
`3]; // ..`, keeps the comment after that item: the JS of that line
opened nothing. Held by the corpus's `array_trailing_comments`, run
against native Rust, `lowering-comment-fields.test.ts`, and the
mutations `statement-comment-on-last-item` and
`closed-lines-past-their-brackets` (`to_oxc/trailing.ts`).

## Amendment: a comment after a drop guard's `let` is after what its `finally` does (#501, 2026-11-03)

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments.

A guard of a closure, `let _restore = Finally(|| depth.set(0)); //
zeroed`, is a `try` whose `finally` is the closure's body (ADR 0865):

```js
try {
  return getter();
} finally {
  depth.value = 0;
} // zeroed
```

The `let`'s line is the JS's `try {` and `depth.value = 0;`, and the
rd605 amendment's lines of brackets after them took the `finally`'s `}`
for the `)` of `Finally(..)`, which the JS of the line opened and left
open. **The lines of brackets past the last JS of a comment's line hold
only brackets of the kinds the Rust's line ends closing**, the `]` of
`32.0];` for `];`, so a block's `}` after code whose line ends `));` is
not its line's: `depth.value = 0; // zeroed`, as the closure's own
comment is, `depth.value = (depth.value - 1) >>> 0; // one less`. A
struct literal's `};` and an array's `];` are as before, and a field of
a struct the JS of its line closes itself, `range: Range { start: 3 },
// range`, stays after it, not past the outer object's `};`, as the JS of
its line opened nothing (the corpus's `array_trailing_comments`, which
holds `closed-lines-past-their-brackets` now that a `)` no longer
reaches a `}`). Held by `lowering-drop-guards.test.ts`, "a comment after
a guard's let is after what its finally does", run, and the mutation
`closed-lines-of-another-kind` (`to_oxc/trailing.ts`).

## Amendment: a comment after a promoted array argument is after it (rd525, 2026-11-02)

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments.

`visit(id, &[""], // path` printed `// path` after the call's `);`:
rustc promotes the array of constants to one, whose JS the map places at
its start and at its `]` only, from the argument's `&` and its `]`, and a
line comment's place was found by the nearest JS before it on its line
that is a name, a number or a string, none here. **Of a line comment's
end, a closing bracket the map places counts too**, so it's after `[""]`,
`[""], // path`, its line's rest on the next, as after `"root", // name`.
A block comment inside an expression still goes after a name, number or
string only. Held by `lowering-comments-items.test.ts`'s "a comment after
a promoted array argument is after it", run, and the mutation
`promoted-argument-comment-past-call` (`to_oxc/trailing.ts`).

An array that isn't promoted, `&[key], // path`, still lost its
comment: its JS had no place of its own, and the formatter, laying the
call's arguments a line each, carried the place of the argument before
it, `id`'s, onto its line, so no JS of the comment's line was there to be
after. Fixed below.

## Amendment: an array a statement makes is where its Rust is (2026-11-02)

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments and the
source map.

A value a MIR statement makes is placed where that statement's Rust is
(`mir_store`), so a call's argument a temporary holds, `id`, maps to its
line. An array or a tuple MIR makes is a list of its items until it's
read (`Value::List`, so `format_args!`'s arguments and a tuple's fields
can be read one by one), and had no place: `[key]` of `&[key], // path`
was JS of no Rust, and its comment was lost. **A list is placed where the
statement that makes it is, as any value is, and its array, once it's
JS, is there**: `[key], // path`, as upstream's `[key], // path`. Of an
array that is a block's last value, `[9, 9, 9] // ..`, the statement is
on one line, as written, `return [9, 9, 9]; // ..`, where the formatter
split it to keep the comment after its last item. Held by
`lowering-comments-items.test.ts`'s "a comment after an array argument
made of a variable is after it", run, the corpus's
`array_trailing_comments`, and the mutation `made-array-unplaced`
(`lower/mir/assign.ts`).

**A constant is placed where it's written, wherever it is.** ADR 0630's
amendment placed a constant only as a call's argument (`argument_spans`,
a fact of the body's calls), and ADR 0708's a static so too, because a
tuple's had no place: a comment above `(s, false)` went before `false`,
inside its array. The tuple's array now has its statement's place, which
that comment is before, so the restriction made nothing: placing every
constant the program wrote, none a macro's, the corpus, the snapshots,
the guide and React's ports print the same, and its mutation,
`constant-placed-anywhere`, survived every comment test. It's removed,
and `argument_spans` with it: a constant is placed as any value a
statement makes is, `operands.rs`'s `written_span`. Numbers and strings
were already where written; what moves is a `bool`'s, a unit's or a
static's place in the source map. "a comment before a tail tuple with a
constant is never inside its array" holds it still.

## Amendment: a comment isn't past a whole match arm, and an inner statement's are its own (2026-11-02, rd630)

A comment above a match arm whose value has no start of its own, `//
c\n Ok(h) => h,`, was attached to the next arm's start, past the whole
arm. It's the arm's: **a comment isn't attached past a whole arm**, and
it's inside the statement the arm's value is of, before the innermost
statement it's in. **Where that statement starts where one it's in
starts, its comments are at a place of its own**, mapped to nothing,
since oxc prints what's at a start before the first node there, the
outer one. The case, a `try`'s value of a `match`'s `Ok` arm, and its
tests are in [ADR 0637](0637-a-match-of-a-throwing-call-is-try-catch.md)'s
amendment of the same date.

## Amendment: a module's `//!` is above its JS (2026-11-03, #486)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a comment's place.

A module's inner doc comment, `//!` or an inline `mod`'s `/*!`, was placed
as any comment is, before the code it's written above. Above a `use`
that makes an import it was above that import; above a trait, a struct
or a `use` of a type, all with no JS, it was lost wherever something
with a `{ }` or a `;` came between it and the first statement, and where
the module's imports were all unwritten, the runtime's helpers or a
binding's, it was below them. The pair `unit_struct_of_instance`'s app
lost its `//!` when its first item became an impl, and the pilot's
`api.rs`, whose first JS is an enum's methods, would have had its own
inside `export const Failure = {`, before `message`.

Rust writes an inner doc comment before everything in what it's in: it
documents the module, never an item. **A module's own `//!`s are its JS
file's description, above its directives and imports, right below
rust-js's line and the file's leading comments**
(`to_oxc/header.rs`'s `Docs`), each as JS writes it (`comments::in_js`),
a blank line between two where the Rust has one, and one after them
where the Rust has one right after them. They're no statement's or
import's comments. Which `//!`s are the module's is the lexer's to say
(`lower/sources.rs`): each inner doc comment carries where what it's in
ends, `Comment::inner`, the `}` that closes it or its file's end, so a
module's are those that end where its code does (`LoweredModule::code`),
an inline `mod`'s are its own and not its parent's. One inside a
function's block, which rustc allows and nobody writes, is placed as
before. Names run 44 chose this, a reversible call, over keeping each
`//!` before the first code with JS after it, which put the pilot's
inside an object and left builtins' and web_forms' below their imports.

The snapshots async, builtins, imports, throws, todomvc and web_forms and
the pilot's committed JS are re-blessed: builtins', web_forms' and the
pilot models' `//!` move above their imports, the others' are kept where
they were lost.

- Held by `test/lowering-comments-items.test.ts`, "a module's inner doc
  comment is above its JS", run: above a trait and its impl, once; an
  inline module's `/*!`, past its struct, in its file and not its
  parent's; above a type's `use` and an enum's methods, two runs and the
  blank lines between and after them; and right above a `use`, no blank
  line. Mutations `module-doc-lost`, `module-doc-of-an-inner-module`,
  `module-doc-also-a-statements` (`to_oxc.ts`), `module-doc-always-apart`,
  `module-doc-never-apart`, `module-doc-runs-joined`
  (`to_oxc/header.ts`), `inner-doc-not-inner` and
  `inner-doc-not-closed-by-its-brace` (`lower/sources.ts`). Since a
  module's `//!` no longer reaches them, `inner-doc-printed-from-source`
  is held by a function body's own `//!`, and 1063's
  `header-comment-blank-dropped` by a plain comment and a blank line above
  a re-export's `pub use`.

## Amendment: a module's `//!` is one JSDoc, its file's description (2026-11-03, rd644)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a comment's form.

#486 printed a module's `//!` lines as JS's plain `//` lines, as rule 3
had every inner doc comment, JS having no doc comment of what it's in.
Upstream's files describe themselves in a JSDoc: React's ReactNoopServer,
ReactNoopFlightServer and ReactDOMServerExternalRuntime start `/**\n *
This is a renderer of React ..\n */`, which their ports write as `//!`
lines, and every port file whose `//!` has an upstream counterpart (all
three; the other 33 `//!`s in react-in-rust are crate roots of their own)
writes it so. The JS printed `//` lines where upstream has the block.

**A module's own inner doc comments are one JSDoc** (`header::docs`): a
run of `//!` lines one `/** .. */`, as a `///` run is its item's
(`comments::jsdoc`), runs a blank line apart its paragraphs, ` *` between;
an inline module's `/*! .. */` its `/** .. */`. Where a line holds the
`*/` that would end the block, each line as JS writes it, `//`, as an
item's doc does. An inner doc comment in a function's block stays a plain
comment, as rule 3 has it: it describes no file.

ReactDOMServerExternalRuntime's JS also starts with a license, `//
Copyright (c) Meta Platforms ..`, which upstream's file lacks: its port's
Rust starts with that comment, and a file's leading comments are its JS's
first, as written (ADR 0383). That's the port's to drop, not the
compiler's.

The snapshots async, builtins, imports, throws, todomvc and web_forms and
the pilot's committed JS are re-blessed for their JSDoc.

- Held by `test/lowering-comments-items.test.ts`, "a module's inner doc
  comment is one JSDoc, its file's description", run, and the tests #486
  names; mutations `module-doc-plain`, `module-block-doc-plain` and
  `module-doc-runs-joined` (`to_oxc/header.ts`).

## Amendment: an index loop's item is the loop's, before its body's comments (2026-11-03, #526)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.

A loop over a slice counts its indices (ADR 1042) and binds its item
first, `const x = xs[i];`, with no Rust of its own. So, as rule 6's
temporary, it was printed at the start of the body's first statement,
after that statement's comments: `// Big ones.\n const x = xs[i];\n if (x
> 3)`. The same happened to a comment above a `match`'s first arm that
does nothing, which the `if` is placed at (#508), `// Next.` of `Ok(())
=> {}`. But that binding isn't the statement's temporary. It's the
loop's pattern, which the program writes in `for x in xs`, before the
body. In a `for…of` the comment is inside the body, above its statement.
**An index loop's item is placed at its loop's `for`** (`loops::item_at`),
that of a loop stepped from its end too. A comment above the body's
first statement is after it, `const x = xs[i];\n // Big ones.\n if (x >
3)`, as a person writes it. A comment above the `for` is still the
`for`'s, as the `for` is printed first.

Held by `test/lowering-comments-inside.test.ts`'s "a comment above a loop
body's first statement is after its item", run, and the mutations
`loop-item-a-temporary` and `loop-from-the-end-item-a-temporary`
(`mir/tidy/loops.ts`). The corpus cases matrix_strings and
multiply_by_a_constant are re-blessed, each a comment after its loop's
item.

## Amendment: a comment in or above a later arm keeps its `else if` (2026-11-04, #595)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.

A `match` whose tree is a chain of its arms (`decisions`, ADR 0364)
places each `else if` of a later arm where that arm's code is, so a
comment before the arm is its value's (the amendment of 2026-10-25
above). That start is the block's first statement's too, and oxc prints
what's at a start before the first node there, the `if`: so a comment
first in the arm's block was taken for one before the `if`, which an
`else` keeps its block for (the amendment of 2026-10-26), and the chain
nested:

```js
} else {
  // Two of two.
  if (a === 2 && b === 2) {
    seen.push(true);
  } else {
    hit = false;
  }
}
```

A comment above such an arm, `// c\n (2, 2) => {`, was before the test's
right operand, which the pattern `(2, 2)` is where, `a === 2 &&\n //
c\n b === 2`; and above an arm that opens a block whose `else if` MIR
placed at its pattern, the chain nested the same way.

1. **An `if` placed where its block's first statement starts has no
   comments of its own**: the comments there are that statement's, which
   oxc prints them before, and an `else`'s only such `if` is an `else if`
   (`statements::at_its_first`). Only a placement makes this so, never a
   comment the program wrote before an `if` in an `else`'s block, whose
   test starts before its block.
2. **A comment above a match arm is its body's past the arm's opening
   too, `pattern => {`, where that block is an `else if`'s**
   (`openings::arm_test`, of `arms::else_if_starts`' `firsts`), as it
   was past `pattern =>` of a
   value (ADR 0590): before the block's first statement, before a comment
   written first in it, whether the `else if` is placed at its arm's code,
   `decisions`', or at its pattern, MIR's. A `switch`'s case keeps its
   comment above its `case` (rd166), and a chain's first arm its comment
   above the `if`.

```js
} else if (a === 2 && b === 2) {
  // Two of two.
  seen.push(true);
} else {
  hit = false;
}
```

Held by `test/lowering-comments-inside.test.ts`'s "a comment in or above
a later arm of a match's chain keeps its else if", run, and the mutations
`else-if-at-its-first-commented`, `else-if-at-its-first-of-any-start`
(`to_oxc/statements.ts`) and `else-if-arm-opening-unread` (`to_oxc/openings.ts`).
The guide's basics page shows it (`sorted`).

## Amendment: an or-pattern's arm, and a returned match's (2026-11-04, #608, #609)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments and the
order arms are written in; what runs is the same.

Two arms the amendment above left out nested their comment or moved it.

**A later arm of an or-pattern**, React's writeStyleResourceAttribute's
`// Santized URLs\n "src" | "href" => {`, MIR tests `name !== "src" &&
name !== "href"` of, with the arms after it first, so its `if` was
`if (t) { rest } else { arm }`, the arms out of order, and the comment
above the `if`, in an `else`'s block:

```js
} else {
  // Santized URLs
  if (name !== "src" && name !== "href") {
```

1. **An `if` placed at an arm's pattern whose `else` is written before
   its `then` is turned**, `if (!t) { arm } else { rest }`, as
   `written_chains` turns one whose `then` is a chain (`tidy/branches.rs`):
   the arms as written, where one test decides between them.
2. **An arm's test goes on past an or-pattern's alternatives**, `"src" |
   ` (`openings::alternative`), to its last one's `=>`, where rule 2 of the
   amendment above asks of it (`openings::arm_test_starts`): the comment is
   the first statement's of the `else if`'s block, written above the arm
   or first in it. A `switch`'s case's values aren't gone past, each its
   comment's (rd139).

```js
} else if (name === "src" || name === "href") {
  // Santized URLs
  out.push("url");
```

A tuple's alternatives, `(2, 3) | (3, 2) =>`, start inside their tuples,
which this doesn't read past; a comment above such a later arm is still
between its test's operands (until #617, below, which needed no rule of
them).

**A later arm of a `match` whose arms each return**, `(Some(a), _) => a,\n
// c\n (None, Some(b)) => { .. }`, is a guard `if` (ADR 0370) once
`written_chains` turns MIR's `if (x == null) { .. } else { return a; }`.
MIR placed that test where the arm it tests for is, the next one, so the
first `if` was placed after the comment above that arm, and the comment was
printed above it, the function's first statement. **The test `written_chains`
turns is its new `then`'s arm's**, so it's placed at that arm's pattern,
the last of `Written::patterns` from where the `if` is placed to that arm's
code (`written_arm`): an explicit fact of where the arm is, which
`source_spans` places the `if` at, and the comment is above the next
arm's `if`. A `switch`'s test stays where MIR placed it, which the
`switch` reads its arm from (ADR 0668).

```js
export function pick(x, y) {
  // Only the first.
  if (x != null) {
    const a = x;
    return a;
  }
  // Only the second.
  if (y != null) {
```

Held by `test/lowering-comments-inside.test.ts`'s "a comment above a later
or-pattern arm keeps its else if" and "a comment above a later arm of a
returned match is above its if", each run, and the mutations
`or-pattern-arm-after-the-rest`, `written-chain-test-at-a-later-arm`,
`written-chain-test-at-the-last-arm`, `written-chain-switch-test-moved`
(`lower/mir/tidy/branches.ts`) and
`or-pattern-alternative-unread` (`to_oxc/openings.ts`).

## Amendment: a later arm of a tuple's or-pattern (2026-11-04, #617)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.

A comment above a later arm of a tuple's or-pattern, `// c\n (2, 3) | (3,
2) => {`, was between its test's operands:

```js
if (
  (b === 3 &&
    // c
    a === 2) ||
  (b === 2 && a === 3)
) {
```

That was MIR's tree, `decisions` making no chain of it (ADR 0364,
amended, #617): its tests nested `if`s, each placed at its alternative's
value inside the tuples, `2, ` and `3) | (`, none an arm's test or an
alternative the amendment above goes on past. In the chain the arm's test
is one `||` of its alternatives, placed where the whole or-pattern is, so
#595's rule 2 reads `(2, 3) | (3, 2) => {` as the arm's test and opening,
and the comment is first in its `else if`'s block, with no rule of
tuples' starts.

```js
} else if ((b === 3 && a === 2) || (b === 2 && a === 3)) {
  // c
  seen.push(true);
```

A returned `match`'s later arm is a guard `if` (ADR 0370), which
`decisions` placed where its block's first statement is, as an `else if`
it was; but that start was no `else if`'s block's, so #595's rule 2
stopped at the arm's test, and the comment was the test's. **An `if`
placed where its block's first statement is opens its arm's block as an
`else if`'s does** (`arms::else_if_starts`' `firsts`): the comment is at
that start, before the `if` oxc prints there, above the guard `if`.

```js
if (a === 1 || b === 1) {
  return 1;
}
// c
if ((b === 3 && a === 2) || (b === 2 && a === 3)) {
```

A tuple's arm of no `|`, `// c\n (2, 3) => {`, the same: `// c\n if (a ===
2 && b === 3) {`, not `a === 2 &&\n // c\n b === 3`.

Held by `test/lowering-comments-inside.test.ts`'s "a comment above a later
arm of a tuple's or-pattern is the arm's", run, the corpus case
`tuple_or_pattern_arm_comments.rs` against native Rust, and the mutation
`guard-if-at-its-first-unread` (`to_oxc/arms.ts`).
