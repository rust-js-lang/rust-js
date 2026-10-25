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
  mutations `mir-temporaries-unnamed`,
  `finish-drop-scopes-unmade` and `finish-returns-through-finally-kept`.

## Amendment: a stage read again is the runner's too

The merge of 2026-10-24 brought ADR 0705's reading again: once each run
of statements a drop ends is one `try { .. } finally { .. }`, where any
was made, the `names`, `in place` and `chains` stages run again. It ran
them inside the `finish/drop scopes` pass, calling their passes by hand,
so `--tidy-until` couldn't stop in them, their contracts weren't checked
after them, and a trace saw them as the one pass (a review, 2026-10-24).

- **A stage says what's run again after it**, `Stage::again`, and its
  passes say whether it must, `Hints::again`, which `tidy` takes once the
  stage is done. Each is run by the runner as any stage is, named
  `again/<stage>/<pass>`, its contracts checked after it, and listed by
  `--tidy-passes` where it runs.
- **`finish` is two stages**: `scopes`, the parameters taken apart and
  the drops' scopes made, its returns through their `finally`, which asks
  `names`, `in place` and `chains` again; then `finish`, the names given.
  The order and the condition are as they were; the JS is the same, byte
  for byte, across the corpus.
- Held by `test/lowering-unwinds.test.ts`'s "a stage read again is its
  own in tidy's trace", the unit test, and the mutations
  `again-stages-not-run` and `again-stages-unchecked`.

## Amendment: `settled` has no pass of what's past a jump

`settled`'s `unreachable` pass took away what its passes left past a
jump, `return e; break bb195;`, before `nothing after a jump`, its
contract, was checked. Nothing it runs after leaves any now: a build
whose pass panicked where it took a statement away compiled the corpus,
the lowering, JSX and snapshot tests and the ports with none, and its
mutation, `late-unreachable-kept`, survived. The pass is gone with its
mutation; the contract is what says none is left, and a debug build
refuses a body where one is, as of any stage.

## Amendment: each stage's JS runs as Rust does

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a program
prints and how it ends are Rust's after every stage, as they are once
tidy is done.

A stage's contracts say its JS parses: each jump lands, no name is
declared twice. Nothing said it does what the body did. Each stage is a
readability rewrite of a body already correct (ADR 0364), so what it
leaves must be correct too, or a later pass is what makes it so, and a
shape that pass doesn't recognize keeps the wrong JS. An architecture
review (2026-10-25) proposed checking that, translation validation, with
the flag this ADR already has.

- **`test/tidy-stages.test.ts` compiles a corpus case once for each
  stage's end**, `--tidy-until` its last pass, as `--tidy-passes` lists
  them, 16 of them with the stages run again, and runs each as the corpus
  runs it, against native Rust. A run agreed with is kept by what it ran,
  as the corpus's are (ADR 0567). `corpus-cases.ts` is the one reading of
  a case's directives, which the corpus and this test both ask.
- **The suite runs six cases** whose JS tidy changes most, control flow,
  assignments, matches, early exits and drops, about 20 seconds; **the
  nightly runs every case**, a sixth on each of six machines
  (`TIDY_STAGES=corpus`). A control holds that the JS checked is each
  stage's, not the finished JS sixteen times.
- **What it found, the first time it ran**, of a quarter of the corpus:
  `range_steps_known.rs` doesn't finish at any stage before `names`.
  Its `(0..u32::MAX).step_by(1 << 30)` is its bounds' arithmetic only
  once the `range steps` pass has folded it (ADR 0517, amended), and the
  same range held in a variable, given to generic code, or of a start
  tidy can't read, isn't folded at all, and never finishes once tidy is
  done: issue #231. A pass is where a program's termination was decided.
  Until it's fixed in the runtime, where std's is, the nightly's part
  with that case fails, as it should.

## Amendment: each stage says what it leaves

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a check in a debug
build, and one pass made to finish its job; what a program does is the
same, and only an `if` that did nothing is gone.

Eleven of the fourteen stages ensured nothing past what every stage does.
Each now ensures what it was found to leave every body with: seven
properties were checked after every stage of every body of the corpus,
6,324 bodies, and a stage ensures one only where it held of all of them,
and of the lowering, JSX, compiler, bindings and snapshot tests' too.

| Stage | Ensures |
| --- | --- |
| `flags` | nothing after a jump |
| `jumps`, `jumps again`, `loops`, `logicals`, `names`, `in place` | each label named; from `loops` on, no empty `if` |
| `values` | nothing after a jump; no empty `if` |
| `chains`, `written` | nothing after a jump; each label named; no empty `if` |
| `settled`, `scopes`, `finish` | those, and no `if` of a constant |

- **`NO_EMPTY_IF`**: no `if` runs nothing either way. **`NO_CONSTANT_TEST`**:
  no `if` tests `true` or `false`. Each is a `contracts.rs` contract with
  its unit test.
- **What it found**: a library's body kept `if (_2$live) {}` in a
  `catch`, a drop flag's whose branch dropped nothing once `never taken`
  took away the `if (false)` inside it, as `never taken` read a list
  before the lists inside it. It reads them first now, and an `if` of
  nothing either way is its test where that does anything, else nothing
  (`what_an_if_always_runs`, and the mutation `empty-if-kept`, which
  `into_iter_of_a_bound.rs`'s library build catches).
- **Not ensured, for now**: `loop_exit_moves.js` keeps `while (true) {
  if (i === words.length || ..) { break; } .. }`, which a person writes as
  a `while` of the test turned; it's the one body of the corpus that does.
- **What it costs**: nothing measured: a body of 3,000 statements compiles
  in 2.15 s with the checks and 2.14 s without.
