# 1167. A statement's facts are its own, and a statement rebuilt keeps them

Status: Accepted (2026-11-16, #1681's change 3, M4, the review's "statement
facts on the statement; rebuild helpers that inherit span and facts by
default").

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about
where rust-js's JS tree keeps what it knows of an `if`. No output changes.

## Context

Three facts are of a statement, not of a value: an `if` whose `else` the
program wrote (`Known::Else`), a `let … else`'s guard where the program
wrote it (`Known::Guard`), and an `if` the program wrote, never one MIR
makes of a `match`'s arm (`Known::Written`). Each was kept on the `if`'s
test, an expression, so it went where the test went: negated, folded into
a conditional, chained into an `else if`. It took three of the bits of the
word every expression's facts are (`Known`). And a statement rebuilt, `kind.at(span)`, kept nothing of the
statement it was made of but what its test said.

## Decision

A `Stmt` has `facts: StmtFacts`, facts of `Known`'s one catalog, those
of a statement alone (`Known::of_statement`: `Else`, `Guard`, `Written`),
so `--without-known` forgets them and `--knowns-found` counts them as any
fact, with no second catalog. A statement made anew is `kind.at(span)`,
knowing nothing; one rebuilt in its place is `stmt.rebuilt(kind)`, its
span and its facts, or `Stmt { kind, ..stmt }` of one whose kind was moved
out; one made knowing a fact is `.at(span).knowing(Known::Else)`. The
three are set on the statement and read of it, never of its test:

- set by tidy's `written_tests` (`Written`), `value_chains`' unfolded
  chain and its link's `else` (`Else`), `jumps/tried.rs`' guard
  (`Guard`) and `jumps/written.rs`' `else`s (`Else`);
- read by preparation's `else` after a `then` that leaves (`Else`), the
  loop whose head a guard isn't (`Guard`), a `try`'s written `if`
  (`Written`) and `jumps/kept.rs`' `continue` (`Else`).

The three places that put them back on a test rebuilt in its `if`
(preparation's `Tests`, tidy's `negations`, `logicals`' bits test,
`Facts::of_statement`) go: the statement keeps them.

Found on the way, by checking each read against what the test said
(`RUST_JS_CHECK_FACTS`, a debug check of the change alone, gone with it):

- where an `if` rebuilt kept its test's facts but not its own:
  `jumps.rs`' emptied branch and `returned_apart`, `jumps/tails.rs`' and
  `switches/case_ends.rs`' emptied branches, and `js::applied`'s `if` of
  returns replaced, each now `rebuilt` or `Stmt { kind, ..last }`;
- where the test lost a fact its statement kept: a pass that replaced an
  `if`'s test, inlining a variable it read, made one of no facts, so the
  `if` no longer said its `else` or itself was written. The statement says
  so still. Of the corpus's cases it reaches, the JS is the same either
  way.

The corpus, the snapshots, std and the port are as they were. Of the
mutations, twelve are retargeted to the statement's facts; three that
dropped a fact as a test was rebuilt in its `if`, `negation-else-dropped`,
`bit-test-drops-its-else` and `if-facts-lost-to-its-test`, retire with the
code they broke, and `rebuilt-forgets-facts`, a statement rebuilt knowing
nothing of what it was, is held by `stmt_facts`' unit test, as no case of the
corpus's has its JS change of it.

## Consequences

- A fact of a statement is set where the statement is made and kept by
  each rebuild of it, never carried by a value inside it, nor lost when a
  pass rewrites that value.
- A pass that makes a statement of another in its place says so,
  `rebuilt`, so what's known of it goes on by default.
- The review's word of facts left 4 bits when it was written; it's 128
  bits since, of which 66 are used, so no bit is freed for it: the three
  stay `Known`'s, as the one catalog's.
