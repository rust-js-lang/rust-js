# 0444. A comment is in the JS where its code is, and so is a blank line

Status: Accepted. Extends [0383](0383-a-files-leading-comments-are-its-jss.md),
whose rules for what JS can hold are now every comment's, and
[0018](0018-print-with-oxc.md).

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
  yet: one in an `if`'s or a `match`'s branch written as `a ? b : c`, in a
  struct literal's field, between a call's arguments. Oxc's printer holds
  a comment before a statement, or at a line's end; no comment is moved
  where it doesn't belong. The next step is to keep the statement form
  where a branch has a comment, as an arrow's block is kept (7).
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
