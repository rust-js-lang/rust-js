# 1031. Mutations are built many to a build

Status: Accepted. Amends [0093](0093-mutations.md): how a mutant is built,
not what it must do.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this decides how
the mutation runner builds the compiler, not what the JS does.

## Context

The nightly runs every mutation (ADR 0093), about 8,000, on 16 machines
of up to four hours each: 3,208 machine-minutes of the nightly's 3,345
(run 40970679815). Of each mutation's time, 85% was the compiler built
again with it in: 63.6 hours of builds, a median of 28 s each on a CI
machine's two cores, against 10.9 hours of its tests, a median of 1.6 s.
Fewer mutations would find less: 31 to 45 survived each night, different
ones, each a test that stopped guarding its rule.

## Decision

**The runner builds up to 32 mutations into one compiler (`--batch`,
`--batch=1` for one each). Each is in a copy of its site: a function's
body, `if crate::__mutant_is("name") { mutated } else { as it is }`, or a
runtime helper's `include_str!`, of a copy of its file with the mutation
in. The compiler chooses the one it is by its file's name, which the runner
gives each mutation's copy of it, or by `RUST_JS_MUTANT`, which its tests
are given.**

- A mutation goes in a build of many only where it's inside a function's
  body that can choose as it runs: not in a `const fn`, which can't ask,
  nor in one returning `impl Trait`, whose two bodies are two types, nor
  outside any function. Those are built alone, as before.
- A build's sites are each once, with all their mutations, and none inside
  another: a function's mutations and those of a function inside it go in
  different builds.
- A build of many that fails is built again as two of half as many, so a
  mutation that doesn't build alone says so as its own result.
- A build of one is the mutation in place, as before.
- A mutant whose compiler writes another runtime runs its tests while no
  other's run, as the checkout's `runtime/index.js` holds it then; it's no
  longer built again alone at the end.

## Why it's the same answer

With no mutation chosen, a build of 82 (14 of them runtime helpers') wrote
the same runtime module, ran the 1,319 corpus cases to their snapshots, and
passed the lowering, compiler and diagnostics suites, 1,222 tests. A sample
of 24 mutations got the same verdicts built 32 to a build as one each.
`test/ci.test.ts` builds a program with three mutations, two in one
function, and runs it as each one's build and as none.

## Consequences

- A mutation's build is its share of its batch's: 12.8 s for 22 on the
  M3 Max, against 3–7 s each alone; on CI, where one alone was 28 s, a
  batch's build is about as long as one, so a shard's time is mostly its
  mutants' tests.
- A change's mutations, `--changed`, are built the same way.
- The compiler's line numbers in a batch are after its copies: a crash's
  location in a mutant's own build reads otherwise.
