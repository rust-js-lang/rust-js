# 1175. The owner table is complete, and kept so by a test

Status: Accepted (2026-11-19, #2162, Campaign 2 Change E, #2112; Tu: "i
want that comprehensive owner table and have a way to maintain that
efficiently"; the design the lead approved). Step 1: the format and both
checks; the areas' sweeps follow, each in its own commits.

## Context

One day found four questions each answered twice: whether a binding
parameter is a native optional argument (`nulled_params::optional_argument`
beside `bindings::is_optional_param`), whether a value is made knowing its
back (by type, #2157), whether a call is std's own (by trait path, #2058),
and whether an iterator is a `Vec`'s `IntoIter` (by an `advanceBackBy`
method, #2154). docs/architecture.md's "Semantic owners" table was an
index of selected contracts, and its test asked only that the modules and
APIs it named were there. Nothing said that a new answer had an owner, or
that no one else answered an owner's question. Two of its rows were of a
shape the test skipped: one had no API cell, one was split at a `|` in a
code span.

## Decision

docs/architecture.md's "Semantic owners" is the one authority on who
answers what, complete for the front end. Each row is
`| Question | Owner | API | Primitive |`: a question, its owner modules,
the API the others ask, and, where the answer bottoms out in one primitive
(a rustc query, a trait resolution, a type predicate, a runtime mark),
that primitive. scripts/owners.ts implements the rules below and
test/architecture-owners.test.ts runs them, in push's fast phase (about
1.3 s); `bun scripts/owners.ts` reports where the table stands.

**Rows.** Each row has exactly four cells, split at each `|` not written
`\|`. Its Owner cell names one module or more in backticks, each a module
there is; each name in backticks of its API cell is defined in the
sources (a Rust `fn`, `struct`, `enum`, `trait`, `type`, `const` or
`static`, or a runtime JS `function`, by its last `::` segment), or is a
module there is. An owner is a path from `src/` where it starts so; else
`src/lower/<owner>` where that is a file or folder there, `x.rs` covering
`src/lower/x.rs` and every file under `src/lower/x/`, `x/` every file
under it; else `src/<owner>`, by the same rule (`names.rs`, `runtime.rs`,
`runtime/steps.js`).

**Code.** A Rust source's code is its text with `//` and `/* */`
comments, string literals (`".."`, `b".."`, `r#".."#`) and char literals
(`'x'`, `b'x'`) blanked, newlines kept, and everything from the first
`#[cfg(test)]` before a `mod` cut. A runtime JS source's code is its text
with comments and `'..'`, `".."` and `` `..` `` literals blanked.

**Completeness.** A candidate is a function of `src/lower.rs` or a file
under `src/lower/` whose line begins `pub fn`, `pub(crate) fn` or
`pub(in crate::…) fn` (with `const`, `unsafe` or `async` between) in its
code, so not `pub(super)` nor private, whose return type is one of:

- `bool`;
- `Option<T>` where `T` names no JS tree type, `Expr`, `ExprKind`,
  `Stmt`, `StmtKind`, `Pattern`, `Prop`, `Function`, `Value` or anything
  of `js::`;
- an enum declared anywhere in `src/` whose body holds no `(` and no
  `{`, each variant a unit's, or an enum whose name is in some row's API
  cell, by its last `::` segment;
- any of these as `R<…>` or as the first argument of `Result<…, E>`.

Each candidate is either named in a row, which is its name or
`Type::<its name>` in the row's API cell with its file of one of the row's
owners, or marked: a line `// helper: <reason>` among the contiguous
comment and attribute lines (each starting `//` or `#[`) directly above
its `fn` line. Never both. A candidate that's neither is listed in
test/owners-unclassified.txt as `<path> <name>`, sorted, once; the test
fails on an unclassified candidate the file doesn't list and on a listed
line that's no longer an unclassified candidate (classified, renamed or
gone), so the list only falls. Its first count is 1,229 of 1,549
candidates; 320 are named by the rows that existed.

**Exclusivity.** Each token in backticks of a row's Primitive cell is
matched as written, any run of spaces in it matching any run, with a word
edge (no word character or `$` beside it) at an end that's a word
character. It must match the code of no source outside the row's owners,
and the code of at least one of the owners' sources. The sources are each
`.rs` under `src/` and each `.js` under `src/runtime/`.

**The runtime.** Its answers are rows by name (`$sliced`, …), owned by its
`runtime/<file>.js`, and its type guesses that stand in for a mark are
primitives of rows, so only the mark's owner reads them (#2159's
`.items !== undefined` and `typeof it.len`). The checker's return types
aren't asked: the TypeScript the checkout has is 7's, which has no
in-process API.

**Merging.** test/owners-unclassified.txt is a line set to `bun run push`
(scripts/push.ts `lineSet`): two sweeps that each take lines off merge as
sets.

**Rows written before this.** They keep their concept as the Question
cell until their area's sweep phrases each as a question. A second
answerer the sweeps find is filed `kind: duplicated authority` in its
area and routed to the owner, not given a row of its own.

## Consequences

- A new cross-module answer fails at push time until it has a row or a
  helper mark, in its author's change, where the owner is known.
- A row with a primitive is a structural check: a second module reading
  the primitive is the duplicated authority the row forbids.
- test/architecture.test.ts's "each owner and question the architecture
  names is there" moved to test/architecture-owners.test.ts;
  `scripts/adr.ts`'s `ownerRows` reads the rows by the same parser.
- The checker's own mutations, scripts/mutations/owners.ts, are each
  caught: a row's API unread, a name passing under any owner, a helper
  mark unread or read past a blank line, `pub(super)` held, an `Option` of
  a JS tree held, both a row and a mark, a primitive read outside its
  owners or by none of them, matched inside a word, a module's
  submodules not its own, a `\|` split, comments and test modules read as
  code.
- The sweeps: the Mac mini's areas (iterators, drops-unwind,
  modules-linking, readability passes, the runtime) first, then m6-lily's
  (nullability, ranges and numbers, `&mut`s, functions and calls, classes
  and JS objects, declarations).
