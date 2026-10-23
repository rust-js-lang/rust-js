# 0659. Tidy is a list of stages, each with what it leaves a body with

Status: Accepted. Amends [0364](0364-mir-lowering.md),
whose tidy was one function of passes in order.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: how the compiler is
built, which no program observes; the one change in output is a statement
no path reached, gone.

## Context

An architecture review, comparing rust-js with Scala 3's compiler, whose
phases each name what they require and guarantee, found rust-js's phases
sound (analysis, lowering, linking, preparation, printing, planning,
publication, each with an owned boundary type) and named four things to
strengthen, in order:

1. **Tidy's order said nothing of why.** `tidy` was some 380 lines calling
   about 130 rewrites in turn, some twice or more, with facts worked out
   once (`changed`, `closures_set`, `paired_temporaries`, `mentions`) and
   read by passes far below, after others had changed the body. Nothing
   said what a pass needed done before it, nor checked what it left, nor
   let a regression be traced to the pass it first appeared in.
2. **`FnCx` is broad**: lowering methods get every fact and capability.
3. **Invariants inside lowering go unchecked**, where `Linked` and
   `ArtifactPlan` show what a checked boundary catches.
4. **Caps measure size, not coupling**, and some architecture checks read
   text where Rust's visibility and types could hold the rule.

It proposed per-pass timing too, and warned against a framework of phases
or an AST per pass. This ADR takes up the first and says what's next for
the rest.

## Decision

**Tidy's passes are one list, `tidy::stages::STAGES`: stages, each a run
of named passes for one job, `flags`, `jumps`, `values`, `loops`,
`logicals`, `names`, `in place`, `chains`, `written`, `settled`, and what
each leaves checked. `tidy` runs the list and calls no pass itself.**

- **A pass is a `fn(&mut Vec<Stmt>, &Hints)`**, which captures nothing:
  what it reads of the body it works out as the body is when it runs.
  The four facts read across passes are worked out by each pass that
  reads them; the corpus's 834 programs' JS didn't change. The type holds
  the rule; no test reads text for it.
- **Contracts, checked after each stage in a debug build**, the build every
  test runs, and never in a release one: of every body, each jump lands
  (a `break` in a loop, a `switch` or its block, a `continue` in its
  loop) and no list declares a name twice; once `settled` is done, nothing
  follows a jump and each label is one a jump names. A broken one panics
  with the first stage that broke it.
- **`--tidy-until <stage/pass>`** writes each body as tidy leaves it after
  that pass, and **`--tidy-passes`** lists them; `scripts/tidy-trace.ts`
  compiles a crate once per pass and shows what each changed, or where a
  text first appears and goes.
- **No per-pass timing.** Tidy took 0.85% of compile time over 80 corpus
  programs (release build): a flag to time it would buy nothing. Measure
  again if a profile shows tidy past a few percent.

What it found, the first time it ran:

- **A `break` after a `return`** in `interpreter.js`'s output: the last
  stages' jumps left it, and nothing removed it. `settled` removes what's
  past a jump last, as its contract ensures, which made the `unreachable`
  of `chains` redundant: it's gone.
- **One question answered twice**: `labels::targets` and
  `tails::names_jump` were the same function. It's `js::names_jump`, beside
  `js::names_break`.
- **Four passes whose mutation survived**, each before this change too.
  Compiling the corpus with each left out showed which cases it changes:
  `written first`'s and `handles in place`'s mutations now check those
  cases' JS. `tail elses` and `typeof tests` changed no program's JS, in
  the corpus, the examples or the lowering tests: lowering writes a lone
  `typeof` test now, and a later pass makes the `else` `tail elses` did.
  Both are gone.

**Amended: a let chain's binding is its `then`'s.** "No program's JS"
missed one the JSX tests compile, react.dev's TerminalBlock: `else if
let Element(e) = .. && let Some(inner) = js::get(..) && let String(_) =
..` kept its arm in a labeled block, `bb27: { if (..) {..} else { if (..)
{ ..; break bb27; } throw .. } }`. The later pass is `written elses`, which
makes an `if` the program wrote with an `else` as it wrote it, where its
`exit`'s statements are each written in its `then`. Here `const inner =
children.props.children;` was written in the condition, where the chain
makes it, so the `else` wasn't made, and `settle jumps` had no `break` at
a branch's end to drop. A statement written in the condition is the
`then`'s, where the binding's in scope; `tail elses`, which made the
`else` of any block's `if (c) { s; break l; } t`, written or not, stays
gone: one pass makes a written `else`. `test/jsx-types.test.ts`'s let
chain and `test/lowering-branches.test.ts`'s chain whose binding is kept
in its arm hold it, with the mutation `written-else-binding-not-then`. A
pass is gone only once the JSX tests, too, show it changes nothing.

## Next

The review's other three, as rust-js takes them, each a change of its own:

- **Narrow what code is given** where a concept already has an owner:
  tidy's `Hints`, never `FnCx` (held by the architecture test), is the
  model. The next are queries that ask only types and names, given a view
  of `FnCx` that has no way to emit or to change locals.
- **Check invariants at a boundary where a mistake has happened**, not
  everywhere: a placeholder resolved before linking, a fact a later
  phase reads still true. A new representation only where it prevents a
  class of error seen.
- **Measure coupling**: the architecture test's report of which modules a
  dispatcher reaches, to find routing that decides semantics; and a rule
  held by `pub(in ..)` or a type wherever one can say it, the text check
  only where none can.

## Consequences

- A pass is added to its stage's list, named, with what it does in a
  comment; a stage's `ensures` says what it guarantees the next.
- A mutation of tidy leaves out a pass by its row
  (`scripts/mutations/lower/mir/tidy/stages.ts`).
- A broken contract is a panic in a debug build: a test that ran fine may
  fail where tidy was wrong all along, and should.

## Extended: what a pass reads has one owner

What a tree writes of its variables, which several passes read, is
`js::Writes`'s alone, and a tree's walks are written once
([ADR 0670](0670-a-trees-writes-have-one-owner.md)).

## Extended: a dependency between passes is said where it is

Of the `in place` stage's chain, `inlined`, `branch values`, `tables in
values`, `declarations`, `inlined in values`: the second `inlined` reads
in place what `branch values` and `declarations` make a `const` of, which
its row says and `inlined-in-values-off` holds; `tables in values` changed
no JS of the corpus, the examples or the lowering and JSX tests, and is
gone.

## Amended

The fourth review (2026-10-23) asked that a chain of passes whose
repetition looks avoidable be measured before anything is removed. Of
`inlined`, `branch values`, `declarations` and `inlined in values`, each
was taken out alone and the corpus compiled. Taking out any one of them
made the JS read worse in 2 to 10 files. Each is held by a mutation of its
own, and `stages.rs` says what each takes and leaves.


## Amendment: what finishes a body is a stage too

A review (2026-10-24) found eight rewrites after the stages, outside them:
the parameters taken apart, then `named`'s temporaries and flags named,
drop scopes as `try`s, declarations, returns through a `finally`, shadows
and appends. No contract was checked after them, and `--tidy-until`
stopping in a stage still ran them, so what it printed wasn't what the
pass it named had made.

- **They're the last stage, `finish`**, in `STAGES`, in the order they ran:
  `--tidy-passes` lists them, `--tidy-until` stops in them, and stopping
  before them runs none, so what's printed is what the named pass made.
  What they read is in the hints: the parameters, which `finish/parameters`
  takes apart (a cell, as no other pass changes them), the names MIR
  suggests, and the locals the program names. `named` is gone.
- **What's checked holds of what's printed**: the contracts of every body
  after `finish` too, and `finish` ensures what `settled` did, nothing
  after a jump and each label named. The corpus finds neither broken.
- Held by the unit test `finish_is_the_last_stage_in_its_order`, and the
  mutations `mir-temporaries-unnamed`, `scope-lets-not-declared-where-made`,
  `finish-drop-scopes-unmade` and `finish-returns-through-finally-kept`.
