# 1200. A list of what to run matches each entry, or the run fails

Status: Accepted (2026-11-25, the lead's call, via the hulk agent).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A harness's rule;
no program's JS changes.

## Context

The lily agent ran the metamorphic net with its cases given as
`test/corpus/...` paths, in a list `RUST_JS_CASES=i/n:<file>` names. The
net takes a case's name, its file's: no case matched, it ran none, and it
said "25 pass", which were its other tests'. A run of nothing that passes
is the hole lesson 8 of docs/finding-wrong-answers.md, "a program all
refused passes all", closes for a table; here it was a list.

Each runner that took a list checked it its own way, or not at all:

| Runner | Its list | Before |
|---|---|---|
| The corpus, and the nets and stages that run its cases (metamorphic, tidy's stages, the iterator forms, determinism) | `RUST_JS_CASES=i/n:<file>` (`caseShard`, test/support.ts) | none: what didn't match was dropped |
| The metamorphic net's rewrites | `METAMORPHIC_REWRITES` | its own count |
| The crate corpus | `bun scripts/crate-corpus.ts [name ..]` | its own, `has no row of` |
| The Crater | `bun scripts/crater.ts [name ..]` | only where none matched |
| rustc's tests | `bun run test:rustc <folder> ..` | a missing path threw; a folder of no test ran none |
| The nightly here | `bun run nightly <job> ..` | its own, `no job` |
| Mutations | `bun scripts/mutations.ts <name> ..` | its own, `no mutation` |

## Decision

- **One check**, `refuseUnmatched` (test/case-lists.ts): of a list and
  what its runner can run, a set of names or a selector's test, the run
  fails where an entry matches nothing, naming every such entry, how many
  of how many, whether it would run none, and, of an entry given as a
  path whose file's name is one, the name to give.
- **Every runner above asks it**, before it runs anything; the copies are
  gone. `caseShard` checks a list against the corpus's cases whatever the
  runner runs of them: a runner of some, the metamorphic net's rewritable
  ones, is given a list a change's selection gives every runner, so a case
  of the corpus it doesn't run is no error.
- **An empty list names nothing** and isn't refused: `check:fast` gives the
  corpus one where it selected only the corpus file's own tests.
- **A mutation's `-t` pattern** stays held as it was: test/mutations.test.ts
  names one that matches no test of its file, read from the file, and the
  runner's controls refuse a mutation's tests that run nothing.

## Consequences

- A list of paths, a misspelt name, or a case since renamed fails at once,
  saying which and what to write; it can't pass.
- A new runner that takes a list asks the same function; docs/
  finding-wrong-answers.md's lesson 18 says so.

Held by test/case-lists.test.ts' "a list naming a case by its path, or
misspelt, fails, naming each" and "the corpus's case list,
`RUST_JS_CASES=i/n:<file>`, of a path or a misspelt name, fails";
mutations `case-list-unmatched-run`, `case-list-only-none-matched-refused`,
`case-list-path-unexplained`, `corpus-case-list-unchecked`,
`corpus-case-list-held-to-the-runners-cases` (scripts/mutations/case-lists.ts
and support.ts).
