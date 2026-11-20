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
code, or `pub(super) fn` where the file is `src/lower.rs` or a file right
under `src/lower/` (`src/lower/<x>.rs`), whose parent module is the whole
front end, so `pub(super)` reaches as far as `pub(in crate::lower)`; not a
nested module's `pub(super)`, which only its parent's modules see, nor a
private one; whose return type is one of:

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
gone), so the list only falls. Its first count was 1,229 of 1,549
candidates, 320 named by the rows that existed; a top module's
`pub(super)` (the rule's second step, found as the sweep began: the
spelling left 308 answers the whole front end asks out of the table)
added 308.

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

**Merging.** test/owners-unclassified.txt was a line set to `bun run push`
(scripts/push.ts `lineSet`): two sweeps that each took lines off merged as
sets. It went with the list (the amendment below).

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
  mark unread or read past a blank line, a nested `pub(super)` held, a
  top module's `pub(super)` left out, an `Option` of
  a JS tree held, both a row and a mark, a primitive read outside its
  owners or by none of them, matched inside a word, a module's
  submodules not its own, a `\|` split, comments and test modules read as
  code.
- The sweeps: the Mac mini's areas (iterators, drops-unwind,
  modules-linking, readability passes, the runtime) first, then m6-lily's
  (nullability, ranges and numbers, `&mut`s, functions and calls, classes
  and JS objects, declarations).

## Amendment (2026-11-20, #2159): a primitive's replaced guesses

A Primitive cell may name, after the words "instead of", the guesses its primitive replaced: a type's shape that stood in for a mark, such as `.items !== undefined` for `$iter`'s mark or `typeof it.len` for "an `ExactSizeIterator`". Its tokens before those words are the row's primitives, as before. **The tokens after them are its guesses.** Each guess is matched as a primitive is, and must match the code of no source outside the row's owners. Unlike a primitive, no owner needs to read it, since a guess is typically replaced everywhere. A cell may name guesses alone.

So a shape replaced by a mark can't come back in another file unseen. scripts/owners.ts's `INSTEAD_OF` parts the cell, and `OwnerRow.guesses` holds them.

Held by:
- test/architecture-owners.test.ts's fixtures ("a row is four cells", "a primitive is its owners' alone");
- the mutations `owners-guesses-unread` and `owners-guess-held-read` (scripts/mutations/owners.ts).

The first two rows are the runtime's: whether an iterator is an array's, `$arrayIter`, instead of `.items !== undefined` and `.items === undefined`; and how many a `TrustedLen` side has left, `$lengthFirst`, instead of `typeof it.len`.

## Amendment: the table is complete, and the ratchet goes (2026-11-20, #2162)

The sweeps took test/owners-unclassified.txt from 1,229 lines to 0: the
Mac mini's share and m6-lily's, the last five mir/nulled's (a0c190927).
With nothing left to classify, a list of exceptions is a second way to
pass the test, so it goes: the file, its `lineSet` entry in
scripts/push.ts, and `UNCLASSIFIED`. Completeness is strict: every
candidate is named in a row or marked `// helper: <reason>`, and
test/architecture-owners.test.ts fails on any that's neither, naming it.

## Amendment: each row's source (2026-11-20, #2200)

Campaign 2's wrong answers traced to decisions made from weak sources:
facts riding JS nodes, crate-wide facts keyed by type that choose a form,
guesses from JS shape, and answers by type where a MIR local exists. Each
was found one by one, after it gave a wrong answer. Campaign 3 (#2200)
classifies every owner by its source, so the weak ones are found first.

- The table gains a fifth column, Source: one class, the weakest source
  any of the row's answers reads. 1 is MIR or rustc's own results, 2 a
  type predicate, 3 a crate-wide fact keyed by type, 4 a fact riding a JS
  node, and 5 a guess from JS shape. A row mixing classes takes its
  weakest, and after a class of 3 to 5 it names its weak APIs, `4:
  \`held_as\``, rather than being split. Phase 3's reasons why a weak
  source is enough go in the same cell, after the class: one cell, one
  rule.
- `ownerTable` refuses a row of other than five cells, and a Source cell
  that's neither empty nor a class, then optionally `: ` and its text,
  naming its line.
- Empty is unclassified, allowed only while phase 1 runs. `bun
  scripts/owners.ts` and every check:fast run print each class's count
  and the unclassified count. Phase 1's last commit makes an empty cell a
  failure, and phase 3 refuses a class of 3 to 5 without a reason.

Held by test/architecture-owners.test.ts ("a row is five cells") and the
mutation `owners-source-unchecked`.

Phase 1 ended with every row classified: 273 read MIR or rustc's own
results, 30 a type predicate, 22 a crate-wide fact keyed by type, 17 a fact
riding a JS node, and 48 a guess from JS shape. Its last commit refuses an
empty Source cell, held by the fixture's empty row and the mutation
`owners-source-empty-allowed`. A crate-wide fact keyed by a struct's field
counts as one keyed by type, being true of every value of it; one keyed by
a function's parameter, read from its callers' MIR, counts as MIR's.
