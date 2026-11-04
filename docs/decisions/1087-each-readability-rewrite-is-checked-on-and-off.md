# 1087. Each readability rewrite is checked on and off, against native Rust and JS-made reads

Status: Accepted (the user's decision, 2026-11-04: a readability-pass
oracle). Extends [0659](0659-tidy-is-a-list-of-stages-with-contracts.md),
whose stage differential was tidy's alone, to preparation's rewrites and to
the generated programs of [0092](0092-generated-programs.md); and the read
rule of [1022](1022-a-field-of-a-struct-js-made-is-read-as-rust-reads-it.md) and
[0672](0672-a-property-a-test-reads-is-read-once.md), a property JS made
read as often as Rust reads it, to an oracle of its own.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Nothing a program
observes changes: two flags a build never passes, and tests. A build's JS
is the same byte for byte.

## Context

A study of the 83 wrong answers filed from 2026-10-29 to 2026-11-04 found
16 regressions, 12 of them from readability or output-form changes: a
rewrite moved, copied or dropped a read, a drop or a statement on a local
check that no owner held it to. Two families stand out. In F-READ (#452,
#457, #462, #482, #485, #515, #525, #564, and #430), a pass read a place
twice, out of order or not at all; in F-LOOPDROP (#440, #463, #465), tidy's
looped drops moved a drop past what it shouldn't. Generated programs found
all three of F-LOOPDROP; the corpus found none.

`test/tidy-stages.test.ts` ran each of tidy's stages against native Rust
(ADR 0659), on the corpus. Nothing did the same of preparation
(`src/prepare/`), the second half of the readability work, which runs on
the finished tree; nor of the generated programs; and native Rust can't
see a read twice of what JS made, where a getter would answer twice: #430,
#485 and #564 were found by hand.

## Decision

**Each of preparation's rewrites is named, and can be left out. Every
readability variant of a program, tidy stopped after each stage,
preparation without each rewrite that changed it, and both at their least,
runs as native Rust does, of the corpus and the generated programs; and
every variant of a read case logs the same reads of what JS made.**

- **Preparation's rewrites are one list, `prepare::PASSES`**: 51 of them,
  each a name, `tests`, `coalescing`, `do whiles`, in the order a module's
  preparation first makes it. `rust-js --prepare-passes` lists them.
  `--prepare-without <pass>`, any number of times, prepares each module
  with every rewrite but those, and `--prepare-fired <file>` writes which
  rewrites changed something, a line each. What a module needs to be
  printed at all, a `&mut`'s handle written as its getter and setter and
  `this` read apart for it, is no rewrite and is always made.
- **The set is preparation's alone**, a test hook as `--tidy-until` is:
  `prepare::module(module, passes)` takes it, the plan holds it, and
  `prepare/passes.rs` keeps it for the module's preparation (`with`), where
  each rewrite asks it where it would be made: last of its conditions,
  `passes::applies(pass)`; around a rewrite of a block or a module in place,
  `passes::run(pass, target, rewrite)`; or of a value as another,
  `passes::rewrite`. Where firing is recorded, `run` compares what it
  rewrote with what it was, and `applies` says it fired where it's made,
  so a rewrite that changed nothing is never said to have fired. Nothing
  outside preparation reads the set.
- **Why "without", not "until".** Tidy's stages each leave what the next
  requires (ADR 0659's contracts), so only a prefix of them is a body
  another stage can take: `--tidy-until`. Preparation's rewrites each take
  any finished tree, and most are made statement by statement inside one
  walk, `block` and `expr`, where no prefix stops between them. Leaving one
  out is what "a rewrite on and off" is, and names it.
- **Only the rewrites that fired get a variant.** A rewrite that changed
  nothing leaves every later one the same tree, so the JS without it is the
  JS as built; `--prepare-fired` says which did, and the test compiles a
  variant only of those: two to six a case, not 51. That a rewrite which
  fired nowhere leaves the JS as it is, is itself checked, of every one of
  the 51, for `calculator.rs`.
- **The oracle, `test/tidy-stages.test.ts`**, runs each program's
  variants: tidy stopped after each stage (prepare on), preparation without
  each rewrite that fired (tidy whole), and the least, tidy stopped after
  its first stage and preparation without every rewrite. Each runs as the
  corpus runs it, against the program's native run; the same JS of any
  variant runs once (`verified-runs`, its map's line left out). Of the
  corpus (`TIDY_STAGES=corpus`) and of generated programs
  (`TIDY_STAGES=programs`, `FUZZ_START`, `FUZZ_SEEDS`): one that differs is
  reduced, as `fuzz.test.ts` reduces one, to the smallest program the same
  variant still gets wrong, `target/fuzz/seed-<n>.stages.rs`. Unset, it
  runs ten corpus cases.
- **The read oracle, `test/getter-reads.test.ts`**, of read cases,
  `test/getter-reads/<name>.rs`: libraries of the js crate's bindings, each
  `//@ call: <export> <JSON arguments>` a call of what it exports.
  `test/getter-reads-run.ts` gives each object argument as a counting
  `Proxy`, the same one each time it's read, whose every read, write, test
  of a key, listing of keys and call is logged in order, with what each
  call gave; a `{ "$fn": v }` is a function that gives `v`. An array is
  given as an array of what it holds, each counted: how an array's own
  methods read it, `map`'s `constructor` and each index it `has`, is JS's,
  which no getter answers. The least variant's log, the reads closest to
  MIR's, which are Rust's, is what every other variant's must be, and no
  call of it may end in a JS error. Four cases hold #430's `if let` and
  guard, #485's `matches!` in a chain, #564's order of a getter and a call,
  and reads through shadows, loops, `?`, `else if` chains, closures and an
  array's search.
- **Where each runs.** Every corpus case at every variant, nightly, as
  ADR 0659's stages were (`stages`, six machines); the night's 600
  generated programs at every variant (`programs`, six machines of a
  hundred); the read cases in the nightly's suite (`nightly` in
  `scripts/shard.ts`). Per push, only where tidy, preparation or the
  oracles change (`readability-passes.yml`): the ten cases, the read
  cases and the first six seeds; and `check:fast` runs the first two when
  tidy or preparation changed (the user's rule for a new net, 2026-11-02:
  nightly in full, per push only a cheap, path-filtered sample).

What it found, the first time it ran: of the corpus's 1,571 cases at
every prepare variant and the least, and 60 generated programs (seeds
20000 to 20059) at every variant, no wrong answer of a build's JS. One
case's least variant throws, `value_cell_set_after_its_call.rs`: a value
cell's rewrite leaves `const _10 = [_11, _14$1];` of the clones it took
away, which a later stage, or preparation's `unread constants`, removes
(#641). It was listed in the test's `leastDiffers`, a list that only
shrinks, until a tuple of the clones went with them ([ADR 0317,
amended](0317-once-cells.md#amendment-a-tuple-of-a-value-cells-clones-641));
the list is empty, and the case is one of the few the suite runs.

## Consequences

- A rewrite added to preparation is added to `PASSES`, named, and asks
  `applies`, `run` or `rewrite` where it's made; one that asks none is
  made in every variant, and checked only as built.
- A rewrite whose variant is wrong is a P0 of the rewrite that the variant
  leaves out, or of one it fed: the name is where to look.
- The nightly's `stages` job compiles each case a few more times, its
  prepare variants and its least, and a new `programs` job runs; the
  per-push sample costs a push that changes tidy or preparation about the
  minutes `readability-passes.yml` takes (TESTS.md).
- Mutations: `scripts/mutations/prepare/passes.ts` holds that a pass left
  out isn't made and that one that fired is said to; `getter-read-a-path-oracle`
  that the read oracle finds #485 put back.

## Alternatives

- **A `--prepare-until`**, as tidy's: no prefix of preparation stops
  between the rewrites `block` and `expr` make of each statement, so it
  would name a walk, not a rewrite.
- **Every rewrite's variant, fired or not**: 51 compiles a case where a
  few change anything, about ten times the nightly's cost for the same
  answers.
- **A Proxy of every object a corpus case makes**: a struct the program
  builds is plain data, which readability may read as often as it likes
  (ADR 1022), so its reads may differ between variants and say nothing.
  Only what JS made is read as Rust reads it, so the read cases are made
  of what JS gives. Three corpus cases have getters of their own, whose
  calls native Rust counts already.
