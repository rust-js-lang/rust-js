# 0444. A comment is in the JS where its code is, and so is a blank line

Status: Accepted. Extended by [0835](0835-an-object-is-across-lines-where-its-literal-is.md):
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
Amended: an `unsafe` block's opening is no code between a comment and
what it's before, and a `None` field a comment is written on keeps its key
([0280](0280-undefined-fields-left-out.md), amended). Amended
(2026-10-25): a comment before a `let` read in place is the next
statement's in its block, one in a `catch_unwind` closure is in the `try`,
and one before a binary's right operand is after its operator.

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
lost before, are kept too.

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
