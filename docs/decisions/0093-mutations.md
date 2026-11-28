# 0093. Known bugs put back into the compiler, which the tests must catch

Status: Accepted. Extends [0088](0088-corpus.md), [0089](0089-rustc-tests.md) and [0092](0092-generated-programs.md). Amended by [1031](1031-mutations-are-built-many-to-a-build.md): mutations are built many to a build.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A test that passes says the compiler does what the test checks, not that
the test would see it do otherwise. A corpus case can pass because it's
right, or because it no longer reaches the code it was written for. Twice,
generated programs were run against a compiler with a fix taken out, on a
branch made by hand, to see whether they'd find the bug again: once they
didn't, and the generator was taught what it lacked.

## Decision

**`scripts/mutations.ts` puts known bugs back, one at a time, and runs the
tests named for each against the compiler built with it.** The mutations
are in `scripts/mutations/`, one list for each source file, at its path
from `src/`: `crates/rust-js-lower/src/traits.rs`'s in `scripts/mutations/lower/traits.ts`,
so code that moves takes its mutations with it. (Amended: one list for
all, 3,400 lines by its 400th.) A mutation is a
change to one place in the compiler's source, found exactly as it's
written, and what it breaks, as Rust would see it:

| Mutation | Breaks | Caught by |
|---|---|---|
| `element-value-first` | `v[i] = f()` checks `i` before `f` runs | `assignment_order*.rs` |
| `compound-place-read` | `x += g()` reads `x` before `g` changes it | `assignment_order.rs` |
| `i32-wrap`, `u64-wrap` | arithmetic doesn't wrap at the type's width | `wrapping.rs` |
| `index-panic-message` | an index out of bounds panics with another message | `index_out_of_bounds.rs` |
| `copy-on-read` | a `Copy` value read from a place is the place | `copy_mutation.rs` |
| `guard-statements` | a guard's statements don't run | `guard_statements.rs` |
| `crash-after-rejection` | rust-js panics after it says what it doesn't support | `union_const.rs`, `closure_clone.rs` |
| `operand-capture` | an earlier operand runs after a later one's statements | `operand_prerequisites` (`test/semantics.rs`) |
| `union-field` | a union's field is read as a struct's, and rust-js panics | `union_const.rs` |
| `dyn-bound-lifetimes` | a `dyn for<'a>` trait's lifetime is left bound, and rustc panics | `higher_ranked_dyn.rs` |
| `begin-panic-payload` | `panic!(5)` before edition 2021 throws a message Rust never shows | `begin_panic_value.rs` |
| `lazy-rhs-statements` | `a \|\| f(&mut y)` runs what `f`'s call needs whether or not `a` decides | `lazy_effects.rs` |
| `while-condition-statements` | a `while` condition's statements run after its test | `lazy_effects.rs` |
| `at-binding-copy` | a binding after `@` reads in place what the binding before it changes | `binding_after_at.rs` |
| `option-some-rest` | `Some(..)` is taken as `None` | `option_rest_pattern.rs` |
| `size-align-swap` | `align_of` is the type's size | `size_of.rs` |
| `array-repeat-shared` | `[x; N]` of what's changed is one object, `N` times | `array_repeat.rs` |
| `never-loop-value` | a `loop` that never ends, used as a value, is rejected | `loop_values.rs` |
| `static-struct-variant` | a static struct's fields are read one place along | `statics.rs` |
| `static-mut-place` | a `static mut` is read and written as its value, not its `{ value }` | `static_mut.rs` |
| `static-mut-reference` | a `&mut` to a `static mut` is allowed | `static_mut_reference.rs` |
| `atomic-fetch-new-value` | an atomic's `fetch_add` gives the new value, not the old | `atomics.rs` |
| `thread-local-storage-static` | std's storage for a `thread_local!` is taken as a static, and rejected | `thread_local_syntax.rs` |
| `trait-of-no-items-impl` | an impl of a trait of no items, `unsafe impl Sync`, `impl FusedIterator`, is rejected | `marker_traits.rs`, `fused_iterator.rs` |
| `user-deref-impl` | a user `Deref` is rejected | `user_deref.rs` |
| `returned-field-write` | a field of what a call's `&mut` points to can't be written | `user_deref.rs`, `returned_references.rs` |
| `own-pointer-unsize` | a pointer of the crate's own, unsized to a `dyn`, is left as it was | `diagnostics.test.ts` |
| `drop-order` | a scope drops what it owns first first, not last first | `drop_scopes.rs` |
| `drop-after-move` | a moved variable is dropped at the end of its scope too | `drop_scopes.rs` |
| `drop-on-assign` | an assignment doesn't drop the old value | `drop_scopes.rs` |
| `drop-without-finally` | a scope's drops run only when it ends normally | `drop_scopes.rs`, `drop_on_panic.rs` |
| `drop-move-before-operands` | a variable moved into a call is taken as moved before a later operand panics | `drop_operand_panic.rs` |
| `drop-function-in-branch` | a drop function is declared in a branch another call to it isn't in | `drop_functions.rs` |
| `drops-walk-uncached` | what a type drops is found once for each path to it, in exponential time | `nested_generic_types.rs` |
| `closure-stepped-iterators` | a closure's body doesn't find its own stepped iterators | `stepped_nested.rs` |
| `static-mut-shared-reference` | a shared reference to a `static mut` is rejected | `static_mut_shared.rs` |
| `drop-ref-parameter` | a parameter bound by `ref` isn't dropped | `drop_params.rs` |
| `drop-let-value-context` | a `let`'s value is taken as moved whatever its pattern | `drop_params.rs` |
| `drop-deref-temporary` | a temporary dereferenced in place is never dropped | `drop_deref_temporary.rs` |
| `temporary-without-finally` | a statement's temporary isn't dropped as a panic unwinds | `drop_temporary_operands.rs` |
| `temporary-not-extended` | a temporary a `let` keeps alive is dropped at the end of the `let` | `drop_temporaries.rs` |
| `unsize-array-as-dyn` | an array unsized to a slice is taken for a `dyn`, and rejected | `drop_temporaries.rs` |
| `generic-drop-not-given` | a generic function given a value with a destructor isn't given its drop | `drop_generic.rs` |
| `generic-drop-not-passed-on` | a generic function doesn't pass its `dropT` on to another | `drop_generic.rs` |
| `part-move-kept-owned` | a field moved out of a value is dropped with it too | `drop_partial.rs` |
| `pattern-parts-kept-owned` | a part a pattern moves out is dropped with what it's matched against too | `drop_partial.rs` |

- **The tests must pass as the compiler is, and run at all,** so their
  failing against a mutation is the mutation's doing, **and fail with a
  compiler that compiles nothing,** so they're using the one they're
  given. The runner's first version set `RUST_JS_COMPILER` in its own
  `process.env`, which Bun doesn't give a process it starts: every
  mutation survived, tested against the compiler as it is.
- **A mutation that isn't caught fails the run,** and so does one that no
  longer applies, as the code it changed has moved, or one that doesn't
  build: each must be made to apply again, or its tests made to see it.
- The crate is copied to `target/mutants/` and built there, with a target
  of its own, so the checkout's build isn't touched, and only rust-js is
  built again for each.
- It builds a few native programs for each, which macOS makes slow, so the
  rustc tests workflow runs it on Linux with `mutations` (AGENTS.md).
- **A mutation's tests skip the corpus's snapshots** (`RUST_JS_SNAPSHOTS=ignore`):
  nearly any change to the compiler changes some case's JS, so they'd
  catch every mutation, and say nothing of whether what the JS does is
  checked.

## Why

- **It's the claim a green run makes, tested:** that each of these bugs,
  back, turns it red.
- **Each is a bug rust-js had, or one a rule keeps out,** so it's what the
  tests were written for, not a change no program would notice.

## Alternatives

- **A mutation tool, such as `cargo-mutants`:** it changes every operator
  and return it finds, most of which the corpus was never meant to cover,
  and each is a build of rust-js; a list of known bugs is small, and says
  what each test is for.
- **Patches:** they stop applying without saying which line moved; a
  change found as it's written says so.

## Consequences

- A mutation names the code it changes as it's written, so a change to
  that code makes it fail to apply, and it's updated with the code.
- The set is a start: a fix worth keeping is worth a mutation, as its
  corpus case is.
- **A mutation is caught only by a test that failed:** the runner ends as
  it does when one does, and names at least one that didn't run out of
  time. One that ran out of time, or whose tests did, was stopped, or
  failed before any test did is inconclusive, which
  fails the run as a survivor does, without saying it's caught; each
  mutation's log is kept in `target/mutants/logs/`. Found in review: every
  failed process had counted as caught. Later, a test that timed out had,
  then one whose `beforeEach` hook did.

## Amendment: a change's mutations are of what it touches

A check of a change, `--changed=<base>`, ran every mutation of each file
it touched: one line of `lower/jsx.rs` reran its 130, nearly all of code
the change didn't come near, and a day's check took 45 minutes. It now
runs, besides those it adds or edits:

- **those in a function it changes**, the function the mutated code is
  in, found as rustfmt and Prettier lay one out (`fn` or `function`, and
  `}` at its indent); code in none, its lines and ten each side;
- **those whose test it changes**: a test their `-t` names, its comment
  above included, or all of a file's, where it changes what they share,
  above the first; and a corpus case their `-t` names;
- **one that no longer applies**, to say so.

A change elsewhere can still matter to one, as making the code it guards
redundant for its test (`captured-base-copied`, after a lowering change
in another file): the nightly run of every mutation finds that, a day
later at most. Of the checks of the last day's commits, it ran a third to
a half as many. `test/ci.test.ts` checks how it chooses. And they run on as
many machines as keeps each near forty, about ten minutes: `ci:check`
counts them first, and asks for 1, 2, 4 or 8.

## Amendment: a mutation's test builds only what it needs

A mutation is judged by its tests alone, but `compiler.test.ts` compiles
some forty programs and runs native Rust before any of its tests runs, for
the ones that check those programs. A mutation of a test of its own
program paid for all of that, and was inconclusive where it broke one of
them, as `regex-slash-unescaped` did the playground. The 59 of its tests
that read none of that are in files of their own, each building only the
crates: `bindings.test.ts`, the binding language and the js and webapi
crates; `modules.test.ts`, a crate's modules as files; `stable.test.ts`,
plain stable Rust; and `lowering.test.ts`, the rest. `compiler.test.ts`
keeps the 45 that check its programs.

## Amendment: each test shows what it's for

125 tests of the compiler were named by no mutation, so nothing showed
they'd fail if what they check broke: 36 of `compiler.test.ts`'s, 36 of
`jsx.test.ts`'s (behind two mutations that run the whole file), and the
rest across a dozen files. Each was audited:

- **89 have mutations now**, 96 in all, each seen caught by the test it
  names: a rule of the code the test is for taken away. Some were the
  first of their code: the `new`, setter and index-setter forms of a
  binding, `$try`, `$toFixed`'s ties, a library's `drops`, object keys
  quoted.
- **9 are covered**: an existing mutation of what they check, run against
  each alone, is caught by it too.
- **27 check what no mutation can reach**: the react, js and webapi
  crates' types and messages, which rustc checks; plain rustc and Cargo;
  or native Rust itself, as "native Rust ran every case" does.
- **None was deleted**: each checks something no other test does, or
  is one of the covered, which check more than their mutation.

What a mutation can't reach isn't untested: rustc checks the crates'
types where they're used, and the corpus runs each against native Rust.

## Amendment: a test renamed or moved away runs its mutations

A change's mutations were those of the tests it touched as they are now,
so splitting `lowering.test.ts` by concept, and renaming "a match of
constants tests each", ran none of the mutations naming the tests that
went: they named nothing, and the nightly's every shard stopped on the
first. A change's lines are read on both sides of its diff, as the file is
and as it was, a deleted file's too, so a mutation of a test's old name, or
its old file, runs and says so. Held by `test/ci.test.ts`, "the changed
mutations are of the functions and tests a change touches".

## Amendment: two workers a machine, sixteen machines a night

A nightly's mutations took 117 to 214 minutes a shard, 97% of its
machine time, and the run waited for the slowest. On CI's four cores a
shard runs two workers, not one: each build is mostly one thread, and
PR #97 measured 18.4 s a mutation against 23.5 s, 1.28× faster. The
nightly splits them across 16 machines, not 8: a public repository's
standard runners cost nothing, and the Free plan runs 20 jobs at once,
which the nightly's other jobs hold only for their first twenty minutes.
Each mutation's build and tests are timed, `mutation-timings-<shard>`,
for the shards to be balanced by next. The counts a check may split
across are one list, `MUTATION_SHARDS` in `scripts/ci.ts`, which
`test/ci.test.ts` holds the workflows' choices and matrix to.

## Amendment: the shards dealt by time

The timings kept for it weren't read: each shard took every `n`th
mutation by name, so a few long ones, a build of its own or a fallback to
every corpus snapshot, made one shard's day. Of nightly 43650311159, dealt
in turn, one shard took 411 minutes of work, its builds' shares and its
tests, and another 230. Now `plan` picks one run, the newest completed
nightly that kept its `mutation-timings-*` (`mutationTimingsRun`), and
each shard of the check downloads those and deals the mutations by them,
the longest first, each to the shard with the least so far, one it has no
time of taking the median (`balanced`, `scripts/shard.ts`). One run for
all, as each shard choosing its own might choose another and run a
mutation twice or never; a download that fails fails its shard, never
falls back to dealing in turn. Of today's 14,089 mutations and that
night's 10,289 times, the longest shard's work is 279 minutes, not 309.

## Amendment: the plan hands the shards their times

Each shard downloaded the times itself: the nightly's every shard's and
main's last five checks', about 320 KB. On main's check of 6d9fcfb45,
GitHub's storage turned one of those downloads away, `503 Egress is over
the account limit`. gh's retry then extracted into the half-written
directory and failed, `openat timings.json: file exists`, and the shard
failed before it ran anything.

Now `plan` reads the times once: the newest `mutation-timings-merged` of
`main`, one file. It hands them to every shard as its output, each
catalog mutation's time in turn, gzipped (`encodedTimings`), so no shard
downloads them and each deals alike. Where `plan` can't read them, it
says so and every shard deals in turn, alike. Each run's
`mutation-timings` job merges its shards' times over the plan's and
uploads the result for the next plan. It uploads only where it had the
plan's times to merge into, or ran every mutation, so what's merged never
loses what was known. A run's artifacts are downloaded one way,
`scripts/artifacts.ts`, each retry into an emptied directory. The shards
restore the test map from the Actions cache by the key test-map.yml
gives, not from a 7.9 MB artifact each.

## Amendment: a check's mutations are since the last check whose mutations passed

A push's check runs the mutations of what changed since a baseline. That
baseline was `main`'s last finished, non-cancelled check, a failed one
too. So when a check's mutations failed, or never ran because the build
broke, the next check skipped those changes, and main turned green with
the survivors still there. The baseline is now the last check that
passed (`checkedSince`), or one that failed elsewhere while each of its
`mutations /` jobs passed (`mutationJobsPassed`). Main is often red for
unrelated reasons, so only requiring a fully green run would let the
baseline lag far behind. `test/ci.test.ts` holds both. Found by a review
on 2026-10-25.

## Amendment: a runtime mutant's runtime is its tests' own

A mutation of a runtime helper changes what `--runtime-module` writes,
and the JS its tests run imports the helpers from `@rust-js/runtime`
(ADR 0103). The runner used to write the mutant's runtime over the
checkout's `runtime/index.js` while its tests ran, then put it back, and
ran no other mutant's tests meanwhile. Anything else that ran in the
checkout then, the corpus parts and other test files of a `check:fast`,
imported the mutant's runtime. That can fail a check that should pass, or
pass a case under the wrong runtime (#720).

Now each such mutant's runtime is installed in its tests' own target, as
its compiler is its own: `node_modules/@rust-js/runtime` there, with a
copy of the checkout's std beside it, as Node resolves a module's
imports from its real path (`test/packages-of.ts`). Every program its
tests write in that target imports them, and so does the oracle, whose
`$isPanic` must be the same module's to know its panics. A test that
reads the runtime or std by path reads `runtimeDir` and `stdDir`. The
checkout's `runtime/index.js` is never rewritten, so these mutants' tests
run beside the others'. All 1,114 runtime mutations ran in 7 minutes on
the Mac. One survived, `collect-dropping-keeps-collected`, as it does with
the old runner. `test/mutations.test.ts` holds the resolution, and that no
test reads the checkout's runtime or std by path.

A mutant whose `--runtime-module` can't be asked would run its tests with
the checkout's runtime, pass, and be said to survive. That was true before
this change as well. Asked for all 1,114 at once under a load of 24, ten
of the asks were killed by SIGKILL, and their mutants "survived". An ask
killed by a signal is now asked again, up to three times. One that still
can't be asked makes its mutant inconclusive, not a survivor.

## Amendment: a mutant's tests never bless

`BLESS=1 bun run check:fast` passed `BLESS` on to "mutations the change
adds". A mutant's corpus tests then wrote the mutant's JS over the
snapshots, and passed (#729). The runner now unsets `BLESS` for every
test run it starts (`testsEnv`), and `test/mutations.test.ts` holds that.

## Amendment: the Nightly of afdd70519's survivors (2026-11-06, #815)

Each is caught or gone with its code. They were found by running the mutant
against the whole corpus in both builds, `crates.test.ts` and the lowering
files:

- `built-layout-gate-inverted` and `route-through-unheld-supertrait` change
  `in_place_collect_of_a_built_type_relayed.rs` and the crate pair
  `default_where_clause_drops`, which they now name.
- `called-value-copy-back-outside` changes only the JS of
  `mut_method_pointer.rs`, so it's `snapshots: true`.
- `await-pending-taken-for-ready` made `compiler.test.ts`'s setup refuse
  `examples/countdown.rs`, which `--bail` judged inconclusive. It names the
  async snapshot alone.
- `mir-iterable-collected-generic`, `mut-param-made-items` and
  `generic-iterator-ref-boxed-for-impl` changed nothing anywhere. Their
  conditions go with them (ADRs 0140, 0715 and 0099 amended).

## Amendment: a readability owner's mutation its tests miss is judged by its cases' snapshots (#864)

A mutation's tests skip the corpus's snapshots, so one that changes only
how the JS reads was caught only by a lowering test written for its form,
or by `snapshots: true` on cases it named. #847 changed the form of a
guard's scope, and 32 mutations survived at once, each found one by one
and given a test by hand (958f75195).

**A mutation of code whose job is how the JS reads, `readabilityOwners`
(`scripts/mutations.ts`: docs/architecture.md's "Readability and printing"
and "Body readability passes", held to it by `test/mutations.test.ts`),
whose tests pass, is judged by the corpus's snapshots too.** Each case that
reaches its site, by the test map (`casesReaching`, `scripts/testMap.ts`,
ADR 1101), with the working tree's changes since the map's commit, or every
case where there's no map or it can't say, as on CI, is compiled by the
mutant and its JS compared with its snapshot alone (`RUST_JS_SNAPSHOTS=only`,
`test/corpus.test.ts`), run neither natively nor as JS, in parts at once,
each ending at its first case whose JS differs. A case that differs catches
it where the compiler as it is makes its snapshot, which is run then. The
map only narrows which cases are compiled: a case that runs none of the
mutated function's code can't differ, so the verdict is the same with it or
without.

Elsewhere the rule stands: what the JS does is what's checked, as most of
the compiler's mutations break what it does, and a snapshot that differs
would hide a test that doesn't see it. `snapshots: true` still says a
mutation of other code changes only how its named cases read.

**Cost**, on the 12-core M6: nothing for a mutation its tests catch. Of one
they miss, every case's JS compiled alone in 12 parts is 13 s, the most a
mutation that differs nowhere pays; the map narrows tidy's mutations to a
median of 1,805 cases of 1,809 (a quarter of them 126 or fewer), as most of
tidy's functions run for every body, so it's the stop at the first
difference that keeps it short. `scoped-drops-if-uncovered`, one of #847's
32, now names its corpus case alone, which passes with it: the runner's
snapshots catch it, six of its 12 parts each stopping at a case whose JS differs, in 2.1 s of tests.

## Amendment: a mutation of code no program reaches is caught by its unit tests (#915)

Some code is reached by no program the compiler is given, so no corpus
case or `bun test` file can catch a bug put back in it: #895's two clears
of `tidied` in `crates/rust-js-mir/src/tidy/jumps.rs`, whose rewrites no MIR-made
body reaches, are each held by a unit test in that file, and removing
either fails `cargo test`. The runner ran `bun test` alone, so the catalog
couldn't hold them.

**A mutation names its `bun test` tests, `tests`, or, where no program
reaches its code, the `cargo test` filter whose unit tests catch it,
`unitTests`: a test's path or its module's from the crate's root,
`lower::mir::tidy::jumps::tests::a_rest_made_a_branch_is_tidied_as_one`.
One or the other, never both** (`test/mutations.test.ts`, which also holds
each filter to a `#[test]` there).

- Its unit tests are the compiler binary's own: a build of many of them
  (ADR 1031) is one `cargo test --no-run --bin rust-js` in the worker's own
  target, its dependencies shared with the compiler's builds, and each
  mutation's run of that binary chooses it by `RUST_JS_MUTANT`. Mutations
  of unit tests are batched apart from those of `bun test`, so neither
  kind's build builds the other's binary.
- The control is the binary built as the crate is: each filter runs at
  least one test, and each passes. A run that is stopped, runs none or
  fails otherwise is inconclusive; one in which a test fails catches it.
- A change runs it, `--changed`, as any other: where it adds or edits it,
  or changes the function it's in, and where it touches the unit tests of
  a source file of the filter's module, from its `#[cfg(test)]` on, or a
  tests module's file whole.

The two clears' mutations, `ending-returns-chain-rest-tidied-unwalked` and
`ending-returns-alone-rest-tidied-unwalked`, are caught, each by its own
test alone: with their filters swapped, both survive. On the 12-core M6,
the pair took 56 s cold, the compiler and the unit tests' binary built as
the crate is among it, and 2 s once those were built, each test 20 ms;
each built alone, `--batch=1`, 5 to 28 s.

## Amendment: a change to the corpus harness runs its own mutants (#918)

The corpus's tests are its cases, made in a loop, and `testBlocks` finds
no test in `test/corpus.test.ts`. So a change of a line of what they share,
its harness, ran every mutation that names a corpus case as the change's:
5,805, 34 min 27 s on the 12-core M6 (#864's one-line branch), each
mutant built again to ask one thing, whether the harness still catches
what the compiler gets wrong. A change to the harness's helpers,
`test/programs.ts`, `oracle.ts`, `corpus-run.ts`, ran none, though they
decide as much of it.

**A change to the corpus harness runs its own mutants, `test/corpus-mutants.ts`,
not every mutation of a case.** The harness is `test/corpus.test.ts`, each
file of the repository's it imports in turn, and each of `test/` they run
by its path (`corpusHarness`), and the mutants' own. A mutant is the
compiler as it is behind a wrapper that gets one thing wrong in what it
gives the harness, no build:

- what one of its runs prints to stdout or stderr, or how it ends, for each
  of the program's run under Node, the program shipped, bundled and
  minified, and the library's: each edit at the JS's end, run only where
  that run's JS is, and in each JS, so the library's is its program's where
  it was (a library's alone in its own, as the harness runs a library's JS
  only where it differs, and says where that's unsaid);
- a crash, a refusal, and a refusal as a library alone, of a program it
  compiles; of one it can't, `compile-fail`, it compiles it, crashes, or
  refuses with another error;
- how the JS reads alone: caught where the snapshots are checked
  (`snapshots: true`) and where they're judged alone (#864), and **not
  caught where a mutation's run ignores them**, as caught there it would
  say every mutation is.

Each runs, in its mode, on the first case of each set of directives the
corpus has (its kind, edition, native target, and whether its library
differs or is refused: 11 sets today), as each is a way the harness runs a
case, and must be caught on each (or pass on each, the one that mustn't
be). They run on a change to any harness file, and in a run of every
mutation, on its first machine; a change to `test/corpus.test.ts` runs
no mutation of a case, and a change to a case still runs its own.

**Why it's sound:** a mutation of the compiler reaches the harness only as
what the compiler gives it, its JS, its exit and its messages, and the
harness reports it only by a comparison of a run's stream or end, the
snapshot, or a compile's result. A harness change that stops catching a
kind of wrong answer stops catching its mutant on the cases it no longer
compares, and `--changed` fails. Shown by six deliberately broken
harnesses, each failing `--changed=origin/main` with the mutants it no
longer catches named:

| Broken harness | Mutants not as they must be |
|---|---|
| stderr not compared | `node-stderr`, `minified-stderr`, on every case that runs |
| the minified program not run | `minified-stdout`, `-stderr`, `-outcome` |
| a nonzero exit taken as the end it wrote (`test/programs.ts`'s `execute`) | `node-outcome`, `minified-outcome`, `library-outcome` |
| snapshots compared where a mutation's run ignores them | `reads-otherwise-ignored`, caught as it mustn't be |
| outcomes compared of `run-fail` cases alone | `node-outcome`, `minified-outcome`, of the `run-pass` and `run-abort` cases |
| the library's runs not judged | `library-stdout`, `-stderr`, `-outcome`, of the cases whose library differs |

What a harness change does to one mutation alone, past its class, the
nightly run of every mutation still finds, as it finds what a change
elsewhere does to one (above).

**Cost:** 48 s on the 12-core M6, 19 mutants on 2 to 9 cases each, against
34 min 27 s; and a change to a harness helper, which ran nothing, now
runs them.

## Amendment: a change's own mutations, its new files and `--list` (#923)

A change's run, `--changed=origin/main`, was said to have skipped five of
its own mutations (24faf6e52's `map-clear-unguarded`,
`map-clear-always-guarded`, `items-cells-by-object`, `items-never-cells`
and `clone-from-vec-replaced`). It hadn't: the run was `--changed=origin/main
--list | tail -40`, `--list` wasn't a flag, so every chosen mutation ran,
and the 51 rows above the tail, those five among them, weren't seen. Three
real gaps were behind the misreading:

- **A flag the runner didn't know was run past.** It is refused now, naming
  the flags there are. `--list` is one: the chosen mutations, each its name
  and file, and their count, none built or run. 24faf6e52's diff lists 82,
  the five among them.
- **A file not yet added to git was no change.** `git diff` doesn't see
  one, so a new source file's mutations, in a new list, looked as they
  were (none did) and none ran until the change was committed. Each
  untracked file is a change of all its lines.
- **check:fast's "mutations the change adds" read `name:` lines of its
  lists' diff**, a second rule beside the runner's, so one whose `find`
  alone was edited, `items-never-cells`, wasn't run. It asks the runner,
  `ownSince("HEAD")`: those whose list entry isn't as it was.

And the run's last line names those not caught, where a `tail` sees it.
Held by `test/ci.test.ts`, "every mutation whose site a change touches is
the change's": a change of every mutation's site chooses each of the
6,000-odd, and one site alone chooses its own; and one whose `find` alone
changed is the change's own.

## Amendment: THIR's range rules once MIR decides (2026-11-14, #1672)

With e4e24e671, MIR's intervals decide each wrap, and THIR's ranges reach
the output only through the bridge (ADR 1137), at the sites MIR doesn't
prove. Main's mutation run at 9dff06aa6 found 49 survivors, almost all of
THIR's range code. On the bridge build they're each caught or gone:

- **12 are caught by their own tests again,** at sites the bridge decides.
- **4 are caught by test/intervals-bridged.test.ts,** which they now name:
  a THIR fact lost makes a listed site stale. They are
  `ranges-if-else-only`, `inclusive-for-unranged`,
  `ranges-is-empty-unread` and `ranges-not-unflipped`.
- **18 are caught by their own tests,** each test now checking that THIR
  and MIR judge its program's wraps alike (`wrapsDisagreed`,
  test/support.ts, any disagreement listed by name). A THIR rule lost
  shows as a `mir-only-wrap` line; one that claims too much shows as a
  `bridged` line.
- **14 go.** In every program their tests compile, each changes only THIR's
  bounds facts, which have fed only the shadow since f7f5bedf4, and no
  bridged site of the corpus. Their code stays until M4 deletes THIR's
  ranges with the bridge. They are:
  `ranges-within-loose`, `rev-for-unranged`, `ranges-for-unknown`, `value-bound-over-length`, `ne-length-cast-not-seen`, `cast-of-length-unknown`, `cast-of-length-past-isize`, `ne-length-not-below`, `ne-length-any-bound`, `ranges-or-as-and`, `ranges-lt-false-gt`, `ranges-gt-off-by-one`, `ranges-ne-any`, `ranges-lt-at-most`.

`hasher-mut-not-read-through`, the 49th, isn't a range rule, and goes to
its own area (#1724, #1730).

## Amendment: THIR's range rules once MIR decides each cast (2026-11-14, #1672)

M3's step 3 makes MIR's intervals decide each integer cast. THIR's
range-proven casts (`range_casts`) are now the shadow's alone, and 20 of its
range rules' mutations, each caught on `main` only through a cast, survived:

- **2 are caught by test/intervals-bridged.test.ts,** which they now name:
  `ranges-break-dropped` and `enumerate-of-any-slice-fn`.
- **18 go.** Each changes only THIR's casts and bounds facts, both the
  shadow's alone, in every program their tests compile, and no bridged
  site. Their code stays until M4 deletes THIR's ranges with the bridge.
  They are:
  `let-else-present-index-unknown`, `cast-kept-out-of-range`, `utf16-count-any-usize`, `ranges-callees-walked-as-a-stack`, `ranges-each-tell-counted`, `join-unbound-forgets`, `join-one-bound`, `widened-counter-forgets-bound`, `widened-counter-past-bound`, `widened-length-counter-below-type-most`, `enumerate-slice-rebound-kept`, `uncast-out-of-range`, `length-arm-unknown`, `length-arm-guard-passed`, `length-arm-not-next-passed`, `utf16-count-impure`, `string-deref-not-read`, `rest-value-read-unread`.

## Amendment: THIR's still-empty cells once MIR decides each `set` (2026-11-14, #1672)

M3's step 4 makes MIR decide whether a `OnceCell`'s `set` is of a cell still
empty (`analysis/once_empty.rs`). THIR's `set_empty` is the shadow's alone,
so its 11 once-cell mutations survived; they go. Each rule has a MIR
mutation in `scripts/mutations/lower/analysis/once_empty.ts`, all caught,
except "judged once", which MIR's fixed point holds (`mir-once-join-either`),
and "made by any call", as MIR makes a cell known empty only at
`OnceCell::new`. THIR's `once-drop-runs-nothing` moved there with
`drop_runs_code`, as `mir-once-destructor-runs-nothing`. The 11 gone:
`once-set-leaves-empty`, `once-set-through-holder-unforgotten`, `once-set-judged-once`, `once-mention-quiet`, `once-place-unmentioned`, `once-holder-reaches-nothing`, `once-raw-borrow-unread`, `once-made-unknown`, `once-made-by-any-call`, `once-rc-clone-gives-cells`, `once-rc-deref-unfollowed`.

## Amendment: a recursive function's walk (2026-11-14, #1742)

`intervals-given-before-every-call` goes. It held that a recursive function
wasn't walked as given only its outer calls' values before its own call of
itself was seen. That way of walking one is gone: a component that calls
itself is walked to a fixed point now, which `callers-recursion-unknown`,
`callers-recursion-never-forgets` and `callers-recursion-outside-unjoined`
hold.

## Amendment: a local lent `&mut` assigned again (2026-11-14, #1782)

`intervals-lent-away-never-assigned` goes. It removed taking a local out
of `lent_away` when assigned again, and no program shows the difference:
MIR assigns a local through a temporary, so a test of it narrows that
temporary, its root, either way. The rule only adds precision; keeping a
local lent away longer is more conservative, never wrong.

## Amendment: THIR's `is_empty()` test (2026-11-14, #1672)

`ranges-is-empty-unread` goes. It made THIR's `is_empty()` false show
nothing of the slice, so `items[0]` in its `else` would be checked. MIR's
intervals decide that index now (`empties`, of `SliceLength::IsEmpty`),
which `intervals-empty-unknown` holds, and no bridged site's wrap reads
THIR's test: test/intervals-bridged.test.ts, its one test, passes with
it. Its code stays until M4 deletes THIR's ranges with the bridge.

## Amendment: THIR's inclusive `for` once the bridge decides nothing (2026-11-14, #1701)

`inclusive-for-unranged` goes. It made THIR's `for i in a..=b` give `i` no
range, held by test/intervals-bridged.test.ts alone. That list is empty
since #1701 C (66ffa61c8): MIR proves every wrap THIR's ranges did, so
THIR's facts decide no wrap and the mutation changes nothing a test reads.
Its code goes with M4.

## Amendment: the bridge once it decides nothing (2026-11-15, #1701)

`bridge-off` and `bridged-unlisted` go. The first turned off the bridge,
THIR's verdict taken for a wrap MIR doesn't prove. The second dropped the
bridge's `bridged` shadow line. Since #1701 C (66ffa61c8), MIR proves every
wrap the bridge decided in the corpus, and test/intervals-bridged.txt
lists none. So neither changes what a test reads: `flushed`'s `i += 1`,
the first's case, is MIR's now. The bridge and its list go with M4.

## Amendment: tracked source-owner mutations (#1848)

[ADR 1154](1154-source-mutations-run-in-isolated-workspaces.md) extends the
existing runner to isolated ECMAScript/TypeScript module inputs, with actual
load evidence, unchanged-source controls, per-workspace package aliases and
the same lease, lock and target isolation. Generator chains use existing
build workers; pure module tests use existing test slots.

## Amendment: a held guard's `deref_mut` once writes through it are seen (2026-11-15, #1877)

`guard-deref-mut-always-a-copy` goes, and so does the branch it mutated. A
held `RefCell` guard's `deref_mut` forgot the cell's length unless its
`&mut` was only given whole to calls; the mutation made it a copy always.
Its row wrote through that `&mut`, `*items = Vec::new()`, which #1877's rule
for a write through a reference forgets on its own, so the mutation
survived. No row tells the branch apart. A struct field's `&mut` cleared, a
raw pointer's clear and `ptr::write`, and every program of the bounds
matrix's `guard` holder each run as native does, with the same shadow lines,
with and without it. Each way the `&mut` changes the length is another
rule's: a write through it (#1877), a call given it (`guard_lent`,
`called_on`), and the guard lent `&mut` where it's borrowed, which is no
longer guarded. A `deref_mut` of a held guard is a copy now, as its `deref`
is. Those shapes are rows of the bounds matrix's `guard` holder (`stored`,
`pointer`, `pointer_write`) and of the guard shadow test, so a weakening of
any of the three rules is caught.

## Amendment: a check follows the last check's results (2026-11-15, Tu's ask)

A check of `main` ran the mutations of what changed since its last check
whose mutations all passed. One survivor kept that base where it was, so
each push ran every mutation since: 72 of 90 checks in two days took all
16 shards, 67% of CI's runner-minutes.

**Each shard keeps what it found of each mutation, of its commit
(`mutation-results-<k>`), and a check follows the newest completed one
whose every shard kept its results, each of that run's commit: the
mutations of what changed since it, and those it ran and didn't catch**
(`scripts/ci.ts`'s `resultsBase`, `mutations.ts --also`). A result is a
run's own, of the commit it checked, never edited: a run with a shard
cancelled or its results missing, or another commit's, is passed over,
and with none the last green check is the base, as before. A mutation is
caught only where a run caught it, what changed since is the change's as
ever, and the nightly runs every mutation.

## Amendment: a check of a change that touches no mutation runs none (dev-loop change 3)

`plan` says `mutations=false` where `changedSince` finds no mutation the
change touches, its site, its list or its unit tests: a doc's change, a
corpus case's blessing, a port's pin. The check's `mutations` job is then
skipped, not one machine running what the last check didn't catch, of code
as it was, which no change since could catch. Such a run has no shard, so
the next run's `resultsBase` passes over it, as it does a cancelled one,
and follows the last check that ran them, its retry list and all;
`runResults` counts its six runs among those that ran mutations, of at
most thirty. The nightly runs every mutation, whatever `plan` says. Case
([0262](0262-when-rust-and-js-disagree.md)): none; this decides what each
check runs, not what the JS does.


## Amendment: the held guard's `deref_mut` retirement undone, and how a soundness branch is retired (2026-11-15, #1928)

The retirement of `guard-deref-mut-always-a-copy` above was wrong, and it's undone: the branch and its mutation are back. A held guard's `&mut` bound to a local, `let all = &mut *items; all.truncate(1)`, reaches the call as a reborrow of the local, `&mut (*all)`. The general calls-may-write-cells rule skips the guarded cell, and the reborrow lends `all`, so no root leads the call back to the cell. Only the branch forgot its length. Without it, `(items.len() - 2) as u32` under `items.len() > 1` gave -1 on main where Rust wraps. The shapes listed for the retirement never bound the `&mut` to a local, and a list of shapes is no proof that a branch is never needed. The bounds matrix's `guard` holder now has the shape (`bound`), and the mutation names that row.

From here, a branch that keeps an analysis sound is retired only when the native pass shows it equivalent: `bun scripts/ablate.ts --native` of its mutation finds no program of the bounds matrix or the corpus, its snapshots ignored, that runs otherwise than native Rust does without it. It runs on the build that decides with the analysis; for a call's index, step 5's switch build. Listing shapes is not enough. Dead code of THIR's that no decision reads is not such a branch, and is retired as before.

## Amendment (2026-11-15, #1672 M4): THIR's ranges' mutations gone

With THIR's ranges deleted (ADR 1137, M4), their mutations,
scripts/mutations/lower/body_queries/ranges*, go with their code, and the
seven of other files whose code went too: named-constant-signed-as-unsigned,
mir-known-bounds-checked, bounds-join-any, assigned-operator-span-off,
cell-borrow-mut-unrecognized, rc-deref-of-a-cell-unviewed and
row-view-of-any-call. intervals-mir-only-unreported is
intervals-proofs-unreported, of the shadow's `proved-<kind>` lines. A test of
a range rule's wraps says which MIR proves (`wrapsProved`, test/support.ts),
so a rule lost, or one claiming too much, fails it.

## Amendment: a survivor retires code only of a run that compares the corpus' snapshots (2026-11-16)

The runner judges a mutation by its tests against native output; the
corpus' JS snapshots count only where the mutation says `snapshots: true`.
A rule that changes only how the JS reads, a check a proof takes away, a
readable form, then survives every corpus case whatever it does. One did:
`loop_ranges`' slice `IterMut` arm "survived" the corpus' 2,617 cases and
was nearly retired as dead, and `check:fast --all`, which compares
snapshots, showed four cases' JS changed without it
(`enumerate_mut_patterns`, `std_mut_items`, `const_block_uninit_array`,
`writer_sink_arguments_given_on`). So, beside the native pass's rule
above: a survivor justifies retiring code only of a run that compares the
corpus' snapshots, the mutation with `snapshots: true`, or the change
itself through `check:fast --all`. The runner says it of each survivor
whose run didn't, "snapshots not compared".

## Amendment: the snapshot judge places each site by a test map, CI's too, never every case (2026-11-16)

A mutation of how the JS reads whose tests miss it is judged by the
snapshots of the cases the test map says reach its site (#864). CI kept no
map, so each such survivor was judged by every case: the nightly's
`finally-holds-what-may-throw`, "of 2553 cases, every one (no test map of
HEAD or an ancestor of it on this machine)", about 15 minutes, some two a
shard (nightly 43650311159). Now, the lead for the user:

- **CI has a map.** rustc-tests.yml's `test-map` job, which every mutation
  shard needs: of a run of every mutation, the nightly's, HEAD's recorded
  and cached by its run; of a change's, the nearest ancestor's the cache
  kept, HEAD's recorded only where there's none (`kept`; recorded in parts
  since the amendment below). Each shard downloads it.
- **No map fails CI's run.** The judge throws, naming CI's job, where the
  machine has none; it never falls back to every case. A run here records
  HEAD's map itself where the machine keeps none of HEAD or an ancestor,
  once a run as its first survivor asks (`ensureMap`),
  of a clean tree, and says how long it took; of a working tree that
  changes HEAD it fails, saying to commit first (the lead, 2026-11-16).
- **A site the map can't place is a survivor that says what to write.** One
  outside a function's counted code, or of a file changed since the map's
  commit (`casesReaching`: a derived fact of its commit, never a stale
  answer), is judged by no case: its mutation names the corpus case that
  catches it, `snapshots: true`. test/mutations.test.ts refuses, where the
  machine has a map, each readability owner's mutation named only by the
  corpus's native runs whose site the map can't place: three of 1,935,
  tidy stages' `never taken` passes, each now `snapshots: true`.

`finally-holds-what-may-throw`, judged by the map of its commit on the
Mac mini: caught by six cases' snapshots of the 1,099 that reach its site,
10 s of tests.

## Amendment: CI builds its mutants incrementally, each build held to being its mutants' (2026-11-16)

CI's rustc was given no `-C incremental` for rust-js, so each mutant built
alone, one a build of many can't hold, compiled the crate whole: 96–102 s
each (dispatch 43769215091), 562 build-minutes of the nightly's 197 timed
alone mutants, where the Mac's builds, incremental by Cargo's dev profile,
take seconds. Now, the lead for the user:

- **The mutation step sets `CARGO_INCREMENTAL=1`**, every build of the
  shard incremental; the incremental directories are kept out of the
  saved cache, gigabytes of each build's sessions, so a shard's first
  build compiles the crate whole and each after it knows the last one's.
- **Each build is held to being its mutants'** (scripts/mutant-builds.ts),
  as a build that reused what it shouldn't would judge another mutant, or
  the compiler as it is, by these mutations' names: its files are in the
  crate as the mutation wrote them, none the crate's own; Cargo compiled
  the crate again (`--message-format=json`'s `fresh: false`); and its
  binary's hash is no other build's of the run, the compiler's as it is
  among them. A build that isn't is each of its mutations' `INCONCLUSIVE`,
  never split or judged.
- **A run of every mutation, the nightly's, builds 5% of its builds again
  fresh** (`--fresh=0.05`), with `CARGO_INCREMENTAL=0`, and judges each of
  their mutations by both; the run fails where the two verdicts differ.

On the Mac mini, an alone build of the mutants' crate after a one-line
change to `tidy/stages.rs`: 33.8 s with no incremental cache, 3.1 s with
it (three of each). On CI, dispatch 43776546919's diagnosis: the crate built
with no setting, 2 min 0 s, no `-C incremental`; again with
`CARGO_INCREMENTAL=1`, 1 min 59 s, its incremental directories made; and
after a one-line change, 8.5 s.

Here sccache refuses `CARGO_INCREMENTAL=1` ("incremental compilation is
prohibited"), and passes an incremental build through only where the
variable is unset: the runner never sets it to 1, only `--fresh`'s 0, and
the Mac's builds stay incremental by the profile.

## Amendment: CI records the test map in parts, on eight machines, and merges them (2026-11-16)

CI's `test-map` job recorded a map on one machine wherever its cache kept
none of HEAD or an ancestor, the nightly's every night: 38.6 min of check
43789968835, its counting build 251 s and its tests 33.6 min, 7,916
seconds of tests on four cores, the corpus's four parts about 980 s each.
Now, the lead for the user:

- **Recorded in parts.** `scripts/testMap.ts build --shards=N` makes the
  counting build and the recording's plan: the function table, what the map
  says of the checkout's sources, and each test dealt to one part, the
  corpus's files in `N` times the machine's cores of parts. `units
  --shard=i/N` runs part `i`'s tests with that build; `merge` writes the
  one map of every part's recording (`mergeRecordings`), and refuses a
  recording a test of the plan is missing from, or said twice by. `bun run
  test-map` is the same three, one part, on one machine: one recording, one
  merge, whatever the parts.
- **The parts are jobs only where they're needed.** `test-map` asks
  `kept` (`--head` of a run of every mutation): where a map serves it's
  given to the shards as it was, and `test-map-part`'s eight jobs and
  `test-map-merge` are skipped outright, so a push adds none to the free
  plan's 20 at once. Where none does, `test-map` builds, eight parts record
  and the merge caches the map and gives it to the shards, which wait for
  it either way. `ensure`, which recorded on one machine, is gone: `kept`
  is the one question (test/test-map.test.ts).
- **The merge is the map of one recording.** test/test-map.test.ts merges a
  fixture's recording in three parts and in one, byte for byte; and of the
  real tests on the Mac mini, HEAD's map recorded whole, 318 s, and in three
  parts one after another, 101, 90 and 97 s, merged in 1 s, were the same
  map byte for byte but its time made: 22,575 functions, 5,373 tests, 71
  run on every change.

On CI, dispatch 43797597400 of c2ef93b6b recorded HEAD's map in 10 min 53 s
where one machine took 38.6: the counting build and plan 4 min 19 s, the
eight parts 4 min 24 s to 5 min 56 s each (their tests 208–300 s), and the
merge 33 s, 5,373 tests, 73 run on every change, as the one machine's map of
8970d448c had. A push whose ancestor's map is kept restores it as before,
its parts and merge skipped: check 43795629437's job, 40 s.

## Amendment: the held guard's `deref_mut` branch retired by the native pass (2026-11-16, #1929)

The branch the amendment of #1928 put back, a held guard's `deref_mut`
forgetting the cell's length where its `&mut` went any way but whole to a
call, is retired by that amendment's own rule. The general rule now sees
through a reborrow of a local, however many: `lent_through` takes `let
all = &mut *items; all.truncate(1)`'s `all`, and the `&mut (*all)` its
call is given, as lent, so `guard_lent` forgets the length at the call,
where nothing did before but the branch. With it, `bun scripts/ablate.ts
--native guard-deref-mut-always-a-copy` finds no program of the bounds
matrix or the corpus that runs otherwise than native Rust does: "neither".
A write through the `&mut` is one through a `*`, whose root's length any
such write forgets. Three programs beside the matrix, a write through the
reborrow, a call through it and the `&mut` stored and cleared, each with
overflow checks off as the corpus builds it, print what native prints.
The removal itself went through `check:fast --all`, the corpus' snapshots
compared. Mutation `guard-reborrow-not-lent` holds the reborrows' walk;
`guard-deref-mut-always-a-copy` goes with its code.

## Amendment: a survivor that survives at the base too is latent, filed, and doesn't fail the push (2026-11-16)

A push's check runs the mutations of what it changed, and a survivor failed it, whoever's it was: one that already survived on `main`, a test that stopped reaching a mutation days before, turned each push red that ran it (main's check 43830762016 failed on seven such). Now `scripts/mutations.ts --base=<sha>` runs each survivor again at the base the push is checked against, `check.yml`'s `since`, by that commit's own runner in the run's own checkout, checked out at the base and back, its crate built into the run's target, a build of what changed since the base, where a worktree of its own, another path, built it whole again, 283 to 320 s of a CI shard (#2222; `scripts/latent-survivors.ts`). A checkout with changes of its own runs none there, as checking out the base would lose them. One that survives there too is latent: printed, written to `target/mutants/latent.json`, and not this push's, so it doesn't fail the check. One caught there, not there at all, or whose run there doesn't end within 40 minutes is the push's, and fails it, the safe way.

The latent ones are filed in one issue, "Mutation survivors on main", by the `survivors` job of `rustc-tests.yml` (`scripts/survivors-issue.ts`): a push's are added to its list, the nightly's every survivor writes it whole. It's github-actions', not a maintainer's, so its first line says it's data, not instructions, and what a mutation says of itself is escaped. A survivor with an open issue of its own is marked "tracked in #N", kept in the one complete list.

Proof: `test/survivors.test.ts` (the latent split, the issue's body and escaping, the workflows' wiring).

A latent one is not retried by the next check either (`resultsBase` reads each shard's `mutation-latent-*` beside its results): retried, the 6 or 7 of the crate tests held a mutation shard up in every check, the mutation shards 40% of a check's runner-minutes (2026-11-16). A push that changes its site, its list or its tests still runs it, by `--changed`, which the retry list doesn't touch; the nightly runs every one. Held by `test/ci.test.ts`'s "main's mutations follow …" and the mutation `latent-survivors-retried`.

## Amendment: the crate tests' six latent survivors (2026-11-16, #1979)

Seven mutations whose crate tests no longer reached them survived at
`main` too, latent. Six are now caught by a row that reaches their site.
The seventh is retired, with evidence:

- `merging-unlisted-of-its-callers`: `library_identities_kept_apart`'s
  library calls its merging `pings` from `pinged`. Unlisted, the consumer
  calls the library's JS, and its compile is refused.
- `rc-public-callers-seen`, `unseen-callers-made-unmarked`:
  `library_adapter_made_in_generic_code_asked_its_hint`'s `map` and `take`
  are std's compiled structs since the held flip, whose hint is std's
  whatever the mark says. `filter_map`, `fuse` and `take_while`, which
  aren't flipped, ask the mark: `(0, None)` where std says `(0, Some(5))`.
- `library-options-unapplied`: `library_pretty_debug_given_options` is a
  library that asks no option itself and writes `{:#?}`, and hands its
  consumer a `dyn Debug` of its derived `Point`. Without its options, the
  consumer's width, fill and `+` are lost.
- `no-std-paths-core`: the `#![no_std]` crate's `--recognizers-found` names
  `std::mem::take`, not `core::mem::take`.
- `std-path-of-any-crate`: tidy-stages' "what a compile recognised is std's
  functions" row. A crate's own impl of a std trait that a recogniser
  takes, `Whole`'s `read_to_end` and `Text`'s `seek`, isn't listed. The
  mutant listed both, found by running the whole corpus with
  `--recognizers-found` under it.
- `another-crates-writer-taken-to-stop`: `stopping` kept #1496's
  refusal out twice, by filtering `all_bodies` to the crate's own and by
  filtering the writers it runs (`ran`). Each mutated alone survived. Both
  removed, `writers_of_a_dependency_where_a_panic_is_caught` is refused
  (`{:?}` of a `dep::Label` whose `Debug` may stop, #1633). With the
  `all_bodies` filter removed alone, all 182 pairs' and 12 triples' JS was
  the base's byte for byte, and the whole corpus, 2,669 cases, ran as
  native with its snapshots. So that duplicate is deleted. A body of
  another crate's that a writer of the crate's runs may now stop, as it
  isn't the crate's to say it never unwinds. The writers' filter is the one
  guard, and the mutation, moved onto it, is caught by that case.


## Amendment: a runtime retirement's evidence is of the tree pushed (2026-11-16)

Evidence that retires runtime code runs on the tree that's pushed. Main
can move during a long run and add the case that reaches the code: the
filter flip's survivors were run whole, the corpus, the iterator matrix
and the copy matrix's filter rows, all native, and then main gained three
cases that `filter-next-by-next` changes, which the push's own check
found. So the final push's full check, every test it selects, is part of
the evidence, never skipped as a sanity pass.

## Amendment: an equivalent of today's programs that's std-faithful is held by a runtime row with a distinguishing source (2026-11-17)

Since the light families' flips (ADR 1136), a lazy adapter the runtime
writes, `$lazySkipWhile`, `$lazyTakeWhile`, `$lazyMapWhile` and `$lazyScan`,
is made only in place, where its source is JS's own iterator, a `$iter` or
a `$Steps` stepping by `next`. There a mutation that steps the source by
`next` where std's adapter asks its `try_fold` survives every program: the
corpus, the copy and conformance matrices. Such a branch isn't retired,
nor simplified to the mutant's form: it's std's shape, which a source with
its own `try_fold`, a compiled struct given to one some day, tells apart.
It's held by a row of test/runtime-package.test.ts that drives the helper
over a source made there whose own `try_fold` says it's asked. The same of
a branch no program reaches today that's right as Rust is: a
`take_while()`'s hint once its predicate failed, `(0, Some(0))`, driven
directly. Code nothing tests doesn't stay, and nothing is retired that a
route to come would need.

## Amendment: a setup that failed is caught, one that ran out of time isn't (2026-11-17)

Checks 43996182737 and 43999199934 failed with `await-value-unread` and
`await-false-edge-ready` inconclusive. Each mutant made examples/fetch.rs's
JS read `_19`, which nothing declares, and test/compiler.test.ts compiles
it in its `beforeAll`: bun test printed `(fail) (unnamed)`, and `--bail`
ended the run before the async snapshot, a named test, ran. The
Consequences' rule judged a failure before any test inconclusive. That
rule was right of a process that failed for its own reasons, and of a test
or hook that timed out. A setup that threw is different: the same tests
passed as the compiler is (the run's controls, or a source mutation's own
control), so the mutant failed it, as surely as it fails a named test.

A mutant whose tests' setup fails, a `beforeAll` that threw (`(fail)
(unnamed)`) or a `describe`'s body (an error between tests), is caught. A
setup that ran out of time is still inconclusive, as a test that did is.
`judge` (scripts/mutations.ts) says so, and its test holds each form as
bun test prints it. Mutations `mutant-setup-failure-inconclusive`,
`mutant-describe-failure-inconclusive` and `mutant-setup-timeout-caught`
hold each half.

The two mutations had named two `-t`s, of which `bun test` reads the last,
so test/compiler.test.ts's tests ran by the snapshots' pattern. They now
name one, and the catalog's test refuses a mutation with more than one.

## Amendment: a mutation of the crate is retired by `--retire`'s evidence (2026-11-17)

Twice in a day a mutation was called unreachable after its own rows alone
ran: the mark of a lent adapter, and a shadow check whose removal
then failed 8 corpus cases. A mutation's rows are the tests it names, not
everything that reaches its code. So the evidence that retires a mutation
of the crate (`src/`, `std/` and the crate's build, what a mutant compiler
is built of) is the run of everything with it, and not a judgement.

`bun scripts/mutations.ts --retire=<name>` builds the mutant alone and
runs it against:

- the compiler's unit tests;
- check:fast's suite with it as the tests' compiler (`--compiler=<path>`):
  every test file, the browsers' too, the metamorphic sample, and the
  corpus whole with its snapshots, in its own target and runtime, as a
  mutant's tests run.

A test fails with it only where it passes as the compiler is, in the
same environment: each that failed is run again with the compiler as it
is, as a mutant's tests run it, and the compiler's unit tests as the crate
is. Some fail there with any compiler, and say nothing: the compile
cache's tests in a mutant's own target, and a unit test that reads the
checkout's snapshots from the crate's copy. They are listed apart.

Where nothing else fails, it prints the evidence, a trailer:
`Retired-mutation: <name>: with it, at <commit>, …`. Otherwise it names
each test that failed with it and passes as the compiler is. Each of those
reaches the code, and the mutation names it as a row instead of going.
Its first run, of `untagged-bigint-number` (caught), named its two corpus
cases and two tests, and the six that fail in any mutant's environment
apart, in 7 minutes here.

The commit that removes a mutation of the crate while its code stays
carries that trailer. The catalog's test (test/mutations.test.ts) refuses
one that doesn't. It checks each commit not yet on `main`, as `bun run
push` runs it. A mutation that was renamed, or moved to another list,
isn't removed: it is matched by its file and `find`. Nor is one whose code
went with it, a refactor's whose mutation is folded into another's: it
can't apply. The rule's first push found one, a refactor of
crates/rust-js-lower/src/traits/given.rs that removed the line its mutation changed. A script's mutation is outside the rule: its tests run
in a copy of the tree with it, and are its evidence. This supersedes
#1979's evidence by hand above. The runtime retirement's rule, a run of
the tree pushed, still holds of the push.

## Amendment: a retirement's evidence holds React's port too (2026-11-17)

`--retire` ran only rust-js's own suite. check:fast leaves React's port
to CI (`leftToCi`), so a mutant that passed the suite could still break
it. One retirement would have: with its mutant, `js::validate` refused 5
of the port's Flight entries, and only its author's regen by hand caught
it. "Nothing tests it" has to mean nothing.

So `--retire` also regenerates React's port at its pin,
`bun scripts/react-in-rust.ts regen` with the mutant as `RUST_JS_COMPILER`,
and compares its JS with what the pin committed (`portDiffers`). Each file
that differs, or the crates failing to compile, is a row that caught it,
`react-in-rust regen: <file>`. Its control runs first, before the suite:
the same regen with the compiler as it is. Where the control differs, the
port says nothing either way, and `--retire` refuses with `port control
fails: <files>; fix the pin first`. A retirement needs clean evidence, so
the lead decided this isn't counted as silent. The trailer says the port
regenerated as it committed. On the Mac mini a regen takes about 35 s once
the port is fetched; the first trial, `drop-let-value-context` (caught by
15 rows), ran whole in about 12 minutes.

## Amendment: the Nightly holds 14 of the 20 runners, its jobs in lanes (2026-11-17)

The Free plan runs 20 jobs at once across the account, and the Nightly
held all 20 for most of its run. Over three nights (43650311159,
43532160173, 43333093940) it ran 142–168 minutes on 2,119–2,231
runner-minutes, peaked at 20, and stayed above 14 for 74–101 minutes. The
pushes' checks, their attribution and the test map's recordings queued
behind it: a check's `attribute-mutations` waited over half an hour for a
runner on 2026-11-17. The Nightly's latency doesn't matter, but a push's
feedback does.

A scheduled workflow now holds at most `NIGHTLY_RUNNERS`, 14, leaving 6
for the pushes. The number is defined once, in scripts/runners.ts, and
`test/ci.test.ts` holds each scheduled workflow to it by its worst case as
its YAML says:

- the heaviest set of jobs that no `needs` path orders (Dilworth's
  theorem, as a minimum flow), the workflows it calls inlined;
- each matrix job counted as many at once as its `max-parallel` lets. A
  matrix whose size is known only as it runs must have a cap.
- every `if:` counted as running, except one its caller's `inputs` decide
  false, and a job GitHub skips because something it needs was skipped
  and its `if:` asks no status. A `needs.*` result, a status function or
  `github.*` can't be decided, so it counts as running.

The Nightly's jobs run in lanes, each job `needs` the one before it with
`if: ${{ !cancelled() }}`, so a red job doesn't stop its lane:

- three at once: the metamorphic net, then tidy's stages, then the
  generated programs;
- one at a time: Crater's parts, the matrices and the nets of crates and
  copies;
- one at a time: the generated programs' fuzz, then each short net;
- check.yml in two calls: its suite (`part: suite`, two of each matrix at
  once), then its mutations (`part: mutations`, 9 of the 16 shards at
  once, and the test map's parts).

So the shards dealt by time above run 9 at once, not 16. The worst case is
14. The callers' `max_parallel` input reaches each matrix of check.yml,
rustc-tests.yml and test-map.yml, and a push passes none, so its checks
are as before. Measuring which of the Nightly's check jobs a passed push
check had already run at the same commit found 0 to 17 runner-minutes a
night, under 1%, so the Nightly doesn't skip them: a job a push's selection
skipped isn't a pass, and the Nightly audits that selector.

## Amendment: a retirement's evidence holds the crate corpus too (2026-11-18)

`--retire` ran check:fast's suite with the mutant, and check:fast keeps
only its `bun test` jobs when it's given another compiler, so crates.io's
corpus ran with no mutant. A fallback's retirement passed the suite and
React's port while itertools, a row of the corpus, reaches that code:
"nothing tests it" was false for the library crates.

So `--retire` also measures the crate corpus, every row
(`bun scripts/crate-corpus.ts` with the mutant as `RUST_JS_COMPILER`), and
holds each crate's verdict (`verdictOf`: `runs`, `compiles`, `refused`,
`blocked`) to the compiler's as it is (`verdictsDiffer`). Each crate whose
verdict differs is a row that caught it, `crate corpus: <crate> <verdict>
with it, <verdict> as the compiler is`: one that falls, and one that rises,
a refused crate the mutant compiles, which reaches the code too. A run that
fails is caught too, with what it said last. Its control runs first, beside
the port's: the corpus with the compiler as it is, `--check`, held to
docs/crate-corpus.md's bars; where it fails, `--retire` refuses with
`crate corpus control fails: ..; fix it first`. Verdicts, not refusal
messages: which refusal a crate's graph reports first varies run to run.
The trailer names the rows measured.

Cost on the Mac mini: the whole corpus, 100 rows, 123 s with the compiler
as it is after a rebuild and 69 s warm; with a mutant, a build of its own,
under two minutes. The first trial, `async-param-copy-unheld`, ran whole in 777 s,
about three minutes of it the crate corpus.

### Amendment: several retired at once share their controls (#2043)

`--retire=a,b,c` (or the flag again) runs the controls once, the
compiler's unit tests, React's port and the crate corpus as the compiler
is, then each mutant against them, a verdict and a trailer of its own; a
failing control refuses them all, and a mutant that says nothing, its
build or a run, leaves the others' verdicts as they are. Each mutant's
suite runs in turn, and its port regen, crate corpus and failures'
controls beside the next one's suite: the crate corpus's phase left the
machine half busy (load 4.5 on the M1's 8 cores), and nothing they run
shares a crate, a target or a checkout with a suite (the unmutated
compiler they compare with is the controls' build, copied, never built
again where the next mutant builds). Serially, each retirement took
13–28 minutes, about a third of it controls.

### Amendment: a verdict that's recorded is confirmed; a flake is inconclusive, named

A verdict that's recorded is confirmed: a run of every mutation, the
nightly's, whose survivors are listed as latent, and `--retire`, whose
verdict removes a mutation (`recorded`). A change's run, per push, locally
and in CI's check, judges once, as before: a flake that hides a survivor
there doesn't land a wrong answer, only delays the survivor's finding to
the next recorded run, and confirming there cost half again: the 42
mutations of `crates/rust-js-lower/src/drops/inlined.rs` on the Mac mini took 112 s and
104 s judged once, 166 s and 164 s confirmed, as each caught mutant's
failing test runs again and a mutant's compiles are never cached; a
change's mutations are most of CI's minutes.

How a catch is confirmed: a test that failed once with a mutant was its
catch, where the run's controls had passed it as the compiler is. A flake, a test that fails now
and then whatever the compiler, then hid a survivor as a catch: a
2-row `--retire` named tests of `test/native-outcomes.test.ts` and
`test/source-mutations.test.ts` rows of `momentary-alias-read-taken`,
which passed when run again with it. Now, in a recorded run, each test
that fails with a mutant is run again with it (`judgeTwice`): those that failed alone,
narrowed by `-t` to their names (`narrowedTo`), and where none fails, the
mutant's tests whole once more, as one test may fail only after the
others; or whole at once, where what failed was a setup, `(unnamed)`, or a
`describe`'s body, which no name chooses. It's caught only where one test
fails both times. Each passed as the compiler is already: an ordinary
run's controls ran every set first, and source mutations' controls ran
their workspace's. `--retire` runs each test the suite failed again with
the mutant and with the compiler as it is (`failingWith`): a row is a test
that failed both times with it and passes as it is (`byControl`); one that
failed as it is says nothing, as before; one that passed run again with
the mutant is `flaked`. A retirement only flakes failed says nothing, and
gives no trailer: fix them, then retire.

A mutant whose only failures were flakes is INCONCLUSIVE, the row naming
each (`<test> failed with it once and passed run again with it, a flake,
which says nothing of it: fix the flake`). Inconclusive counts as not
caught in the run's verdict, failing the run, the nightly's too, as a
survivor does: a flake named and fixed, rather than a survivor hidden as a
catch or a flake passed over in silence. A test that ran out of time
still says nothing (`failures`, the one reader of a run's failures, which
`judge` asks too).

Amended (#2458): but a mutant's hang, run again. A compile that didn't
finish fails its test (`rust-js crashed: didn't finish in 60s`), and run
again, that test's compiles, each up to its 60 s, outlast the test's own
time, so bun says it timed out and `failures` drops it. Nightly
45309220022's `drops-itself-revisits` was judged that way: it failed three
times, once as a compile that didn't finish and twice out of time, and it
was named a flake that "passed run again", as `drop-of-its-own-type-inlined`
had been the night before. Now, where the first failure was a compile that
didn't finish, the same test out of time run again (`timedOut`) is that
failure again, caught. Any other test out of time run again still says
nothing, and its flake says it "ran out of time run again with it", never
that it passed.

## Amendment: a retirement judges nothing below the disk's floor, and runs each control beside its mutant (2026-11-18)

A `--retire` of `guard-named-twice-kept-whole` and
`guard-closure-beside-itself` ran with the disk at its floor, 40 GiB
free (`floorBytes`, test/bounded.ts). There the loop's stores keep
nothing and some tests fail with any compiler: 6 of
test/source-mutations.test.ts ("a source mutation workspace would cross
the development disk floor") and 4 of test/native-outcomes.test.ts. Each
failed with both mutants. Their control, the compiler as it is, ran after
the port and the crate corpus, about an hour later with more free, and
passed. So each mutant was said caught by tests that say nothing of it.

Two changes, so a run never blames its environment on a mutant:
- **The floor.** `belowFloor` is checked before the controls, before and
  after each mutant's suite, and before each pair of reruns. Below it, the
  run judges nothing and says why, naming the free and the floor: the
  whole run where it starts there, and the one mutant, which says
  nothing, where its suite meets it.
- **The control beside the mutant.** Each file whose tests failed with the
  mutant is run again with it and at once as the compiler is
  (`pairedReruns`), in the suite's step, not after the port and the crate
  corpus. A test that fails under the load or the disk it met fails both,
  and says nothing (`byControl`, as before).

test/mutations.test.ts holds both. Mutations `retire-judges-below-the-floor`
and `retire-control-rerun-apart` are caught.

## Amendment: a matrix's cap reads its input through `fromJSON` (2026-11-18)

Nightly 44216138054, the first since the lanes, ran none of its suite's
test shards, rustc tests' shards, fuzz seeds, test map parts or mutation
shards, and so made no `mutation-timings-merged`. A workflow's input is a
string: the callers' `max_parallel`, '1', '2' and '9', made each
`max-parallel: ${{ inputs.max_parallel || <n> }}` a string, and GitHub
created none of that matrix's jobs, with no error and no job listed; what
needed them was skipped. A push passes none, so its default, a number,
served, and its checks ran as before.

Each of the 7 caps is `${{ fromJSON(inputs.max_parallel || '<n>') }}`.
scripts/runners.ts's `capOf` reads that form, and refuses a
`max-parallel` that reads an input without `fromJSON`, naming the form to
write, so test/runners.test.ts's bound fails on one. The bound is 14 as
before. Mutation `runners-string-cap-taken` is caught. No artifact is
new: the night's test map parts and mutation shards upload and download
what they did before the lanes (the test map's build, 23.5 MB, once up
and once down by each of its 8 parts).

## Amendment: a site no run executes goes with its code, by a coverage or marker build (2026-11-18)

`--retire` builds and runs one mutant at a time, 13–28 minutes each. The
survivors of #1979 and the nightly, 206, were first sorted by one run of
a compiler built with `-C instrument-coverage` over what `--retire` runs:
check:fast's suite with it as the tests' compiler, React's port
regenerated at its pin, the crate corpus and the compiler's unit tests.
A survivor whose site no region of that run executed is caught by nothing
because nothing reaches it, and its code is dead as the suite stands.

So a site no run executes is evidence too, of the code's removal, not
the mutation's alone: its code goes with its mutation, as dead code does,
and the commit says so in one fixed form, which `retiredBy` reads as it
reads `--retire`'s:

`Retired-mutation: <name>: unreached, at <commit>, by <the build and its runs>`

Two limits, each met in the first sweep:

- **A site with no region of its own says nothing.** Coverage is counted
  by region, and an alternative of a match arm, `"TakeWhile"` among an
  arm's eighteen, or a type of a `matches!`, shares its arm's counter: a
  line of it has no count, and "no instrumented line" isn't unreached.
  Such a site is measured by a marker build instead: the compiler with
  each site made a block that appends its name to a file before it
  answers, over the same runs. The three of the first sweep,
  `take-while-folds-unseen`, `scan-folds-unseen` and
  `filter-map-not-back-stepped`, ran hundreds of times
  (`held_filter_map_from_its_back.rs` and `filter_ends.rs` reach the last):
  survivors of reached code, not dead.
- **check:fast with `--compiler` runs the suite whole, but not every
  net.** The corpus with std's iterator structs (`forms`) follows the
  change's selection, which a marker build, itself a change, narrowed to
  1,009 of 2,772 cases; and the std, async, numbers and format nets are
  CI's. The run that says a site is unreached runs each of them whole
  (`bun scripts/iteratorSlice.ts run <net> <parts>`) beside check:fast.

The first sweep removed three sites, each silent in every run above, at
a1b0d7aac, and in rustc's `iterators/`, `for-loop-while/`,
`array-slice-vec/` and `closures/` (179 that pass): `mapped-adapter-hint-refused` and `mapped-hint-refused`
(crates/rust-js-mir/src/iter.rs, a map's `size_hint()` of std's adapter of a
source whose hint is std's: each such map is now a `$Steps`, the unmapped
arm's, or a JS adapter `in_place_adapter_hint` answers, so an unknown one
is refused, as any other), and `in-place-part-copied-held`
(crates/rust-js-lower/src/iterators/clones.rs, a `copied()` cloned in place: a
`cycle()` of one is refused before it, "`cycle()` of a
`Copied<Map<..>>`", and a part cloned apart is std's compiled struct).
`own-bindings-of-its-name-counted`, the fourth of the coverage run, was
already retired. Mutation `unreached-trailer-refused` is caught.

The second sweep (2026-11-23, #1979's 128 survivors at 530abe1e6) built
the compiler counting its crates (scripts/counting-rustc.sh) in release,
over check:fast with it as the compiler, every net whole
(`iteratorSlice.ts run <net> 8`), React's port at its pin, the crate
corpus and `cargo test`. Of the 89 Rust sites, 84 ran; four are rows of a
table, with no counter, left to `--corpus`; one ran in none, and went with
its code: `option-filter-unrecognized` (crates/rust-js-lower/src/recognition/types.rs,
an `Option`'s `filter` as a present item, whose fact MIR's intervals give
since 19246947d). Of the runtime's, `rev-advance-by-stepped`'s helper,
`$revAdvanceBy`, was given only by compiled_steps.rs's `advance` of a
`Rev` whose `advance_by` isn't std's compiled one, but compiled_std.rs
compiles every `Rev`'s: the closure ran, its `filter` dropped it each
time, and the branch that writes the helper counted none; both went.

## Amendment: what caught a mutant is named (2026-11-18)

Each caught mutant's result line names the first test that failed with
it, by its file and its name as bun prints it, `mut-iterator-extension-unpeeled
caught by test/corpus.test.ts > dyn_of_a_lent_iterator_steps_it.rs`; a
unit test's by `cargo test` and its name, a snapshot's by the cases whose
snapshots caught it. `target/mutants/results.json` keeps them as
`caughtBy`, beside `results`, whose verdicts stay as they were for their
readers (`ci.ts`, `attribute.ts`, `latent-survivors.ts`). Without it,
finding which case of a whole corpus caught a mutant, to retarget one whose
named tests stopped reaching it, took a second whole-corpus run by hand
(12 minutes on the M1). `catchingTest` reads it off the run's output:
not a test that ran out of time, and not a `(fail)` quoted in a failed
test's diff. Mutations `catching-test-timed-out-counted` and
`catching-test-without-its-file`.

## Amendment: a mutant of the bindings crates' Rust is compiled by its tests (2026-11-18, #2105)

A mutation of `builtins/src/*.rs`, the js crate's handwritten Rust, isn't
the compiler's: it took the source mutations' path (#1848), whose proof
that the mutant ran is a JS mark of its load appended to its file, and
that path threw "source load proof is not supported for
builtins/src/lib.rs", ending the whole run. `string-replace-all-is-first-only`
(#2089) did, so every run that held it, a nightly's shard or a push
changing that file, ended there.

The bindings crates' Rust (builtins/src, webapi/src and react/src) is
compiled, not loaded: each test that reads them builds them of its
workspace's files into its own target as it starts, `buildReact`, whose
inputs they are. So their mutant is written as it is, with no mark, and
judged by its tests in its workspace as any source mutant is, its control
first; the proof it was compiled is that each test file it names builds
the crates (`unbuiltBy`), and one that doesn't is INCONCLUSIVE, never a
survivor. `string-replace-all-is-first-only` is caught by its test, in
about two minutes here. Mutations `source-bindings-unbuilt-judged` and
`source-bindings-marked` are caught.

## Amendment: a mutant against the whole corpus runs it in parts, and `--corpus` sweeps survivors so (2026-11-18)

The sweep of the survivors the coverage run found reached, 190 of them,
asked each against the whole corpus with its snapshots compared. Each
mutation's tests set to `["test/corpus.test.ts"]`, the runner ran each
mutant's corpus in one `bun test` process, twelve mutants at once: a load
of about 30 on the Mac mini's 12 cores, and every mutant it judged,
eleven by the time it was stopped, ran past its time, inconclusive. Its
controls already ran the corpus in parts.

So a mutant whose tests are the corpus whole, no `-t` (`ofWholeCorpus`),
runs it as its controls do: in `caseParts` parts at once, each its
`RUST_JS_CASES` and its own slot among the tests at once, each to its
first failure (`testsInParts`), passing where every part does; its
reruns too. And `bun scripts/mutations.ts --corpus <name>..` runs each
named mutation so, its snapshots compared, whatever tests it names, and
without its settings, which select other files' parts
(`againstWholeCorpus`): the sweep's one way, the catching case each
caught one names in its result line and `caughtBy` its retarget. No
catalog is edited for a sweep. The corpus in 12 parts took 117 s as the
compiler is here. Mutations `whole-corpus-in-one-process` and
`corpus-sweep-keeps-settings` are caught.

## Amendment: a mutation's time is its share of its shard, and the plan estimates as the shards do (2026-11-19)

Sized by expected time (26af155a2), main's check 44441824204 gave its
change's five mutations 16 machines, 11 of which ran nothing. Two faults.
The plan estimated a mutation no run had timed by the timed ones of its
file among the mutations it chose, where the shards take its file's in
the whole catalog; two new mutations of `scripts/ci.ts`, none of whose
file's twelve timed ones was chosen, were each the catalog's 90th
percentile, 987 s, and one such never fit a shard. And that percentile
was a nightly's wait, not its work: a mutation's tests were timed from
their asking for a slot, so a nightly's, behind thousands of others,
recorded a median of 482 s where a check's run of the same took 2 to 8.

Now each mutation's time is its share of its shard (`shareOfShard`): its
build's own seconds over the shard's build workers, and the seconds its
tests held a slot (`slots`), never those they waited for one, over the
test slots. A shard's mutations' shares sum to what their part of it
takes: of 40 mutations of `backs.rs` here at CI's two workers and four
slots, 48 s of shares for 54 s of the mutants' phase. The plan and the
shards estimate an untimed one alike, by the catalog's `mutationFile`.
A shard is expected to take its setup, the compiler as it is (120 s,
87 to 130 s of 44441824204's shards' `phases.json`), its mutations'
shares and a browser's install where one drives one, within 12 minutes,
about the suite's shards' 11 and 15. The merged times are a new artifact,
`MERGED_MUTATION_TIMES`, named in `scripts/ci.ts` alone and uploaded by
the name the merge says, so the old one's waits are never read: until a
nightly times every mutation again, each untimed is `UNTIMED_SECONDS`.
The check of 44441824204's mutations with the times its plan read now
takes one machine. Mutations `mutation-shards-grouped-by-the-chosen`,
`mutation-shards-compiler-uncounted`, `slot-held-from-asking` and
`share-of-shard-unshared` are caught.

## Amendment (2026-11-20): a push's type-check applies those no build can choose between together

A push type-checks its change's mutations (`--typecheck`). Those in a function's body share one check, each chosen by name as the compiler runs. Each of the others was a check of its own: a constant table's row, a signature, a `const fn`'s body, which no `__mutant_is` arm can sit in. That was 50 of a sweep push's 52 checks, 93 s for 943 mutations.

**They're now applied at once, in batches whose `find`s overlap in no file (`together`).** A batch that fails is halved until each failure is alone and named, as before. The same push's set is 1,106 mutations in 8 checks, 22 s.

This can mask an error. One mutant can remove the only use of what another removes, so the batch type-checks where one alone wouldn't. That's accepted because CI's shard builds each mutant alone. There, a mutant that doesn't build says plainly that it was built alone, and that a push's type-check applied it together with others, which can hide its error (`aloneProblem`).

Held by test/mutation-changes.test.ts's "a change's mutants are type-checked many to a check", and the mutations `typecheck-unsited-each-alone` and `built-alone-unsaid` (scripts/mutations/runner.ts).

## Amendment (2026-11-21, #2206): the runtime net runs against the mutant's runtime

test/iterator-steps.test.ts, the runtime net, imported `../std/runtime.js`,
the checkout's. So a runtime mutation it alone could catch passed it
anyway. The runner's control said so: the file passed "with a compiler that
compiles nothing", and the runner refused it as a mutation's test. Runtime
code no program reaches apart had no mutation as a result: `$someItems`'
`next_back` and its hint once it's ended (#2205).

The net now imports `helpersModule` (test/support.ts), the
`@rust-js/std` runtime the tests' JS imports. That's the mutant's own where
the runner installs one in its target, the checkout's otherwise, and an
empty module for the control, where the net then fails. A runtime mutation
the net catches is caught, and the control still holds the net to the
runtime it's given. `some-items-back-by-its-filter` and
`some-items-hint-unended` are the first two caught this way, by the net
alone. Any other test that holds runtime code imports it the same way.
test/std-fresh.test.ts reads the checkout's on purpose: it checks the
checkout.

## Amendment: a failed control leaves its own mutations unjudged, not the shard's (2026-11-22, #2304)

Nightly 44963035271's 16 mutation shards all failed, and 4 of them, 5, 9,
10 and 12, judged none of their ~950 mutations each, a quarter of the
night's: the runner checks each mutation's tests as the compiler is first,
and one set that didn't pass threw, ending the shard. Shards 5, 9 and 10's
was `test/match-trees.test.ts`, failing on the night's commit itself
(`match-trees-labeled.txt` behind what main labeled; main's own check was
red with it), and shard 12's a Vite watcher test slower than its 44 s
under the shard's load. **A test set that fails as the compiler is, runs
nothing, or passes with a compiler that compiles nothing, and a unit-test
filter that doesn't run and pass, now leaves only its own mutations
unjudged** (`unjudgedBy`, by `testSetKey`): each is `INCONCLUSIVE`, said
with what its tests printed, and every other mutation is still built and
judged. An inconclusive mutation is missed, so the run still fails, as
loudly; only a run with nothing left to judge throws, as before. Held by
`test/mutations.test.ts`, "a failed control leaves its own mutations
unjudged, not the shard's"; mutations `failed-control-unjudges-all` and
`failed-unit-control-unread` (scripts/mutations/runner.ts).

## Amendment: a run of every mutation files its survivors and fails only on what it couldn't judge (2026-11-22, #2304, the lead's decision)

A run of some mutations, a push's or the ones named, fails on a survivor
that isn't latent at its base. A run of every mutation, the nightly's
(`recorded`), has no base to spare one by, and its every survivor already
goes whole into "Mutation survivors on main" (`scripts/survivors-issue.ts
nightly`, of each shard's `results.json`). Failing on them too left
nightly 44963035271's every shard red on 4 to 11 survivors each, so a
shard whose harness broke looked like the others. **Such a run now fails
only on what it couldn't judge** (`failingOf`): an inconclusive or unrun
mutation, a corpus-harness mutant out of place, a fresh build that judged
otherwise. Its survivors are said, counted, written to `results.json` as
before, uploaded and filed by the `survivors` job, which runs however the
shards end (`always()`). A red run of every mutation means the harness
broke. Held by `test/mutations.test.ts`, "a run of every mutation fails on
what it couldn't judge, not on its survivors"; mutations
`every-mutation-survivors-fail` and `some-mutations-survivors-spared`
(scripts/mutations/runner.ts).

## Amendment: a shard's tests as the compiler is are dealt with its mutations (2026-11-22, #2304 (c))

A shard runs its mutations' tests as the compiler is before any mutant,
each test file once by mode, 599 to 1,119 s of nightly 44963035271's
shards, and the deal left that out: each mutation went to the least-loaded
shard by its own time, so one test file's mutations were spread across
shards that each ran its control, and a check's plan added the control's
estimate only after dealing. **Each mutation's control runs are now keyed
costs** (`controlRuns`, in `scripts/mutations.ts` beside `testSet`: a test
file's run by mode, its whole run, each corpus case its `-t` names, at
`TESTS_AS_IT_IS`'s seconds), **counted once a shard in its load**, and a
timed mutation goes where its time and the runs it adds there are least
(`dealtByGroup`'s `controls`), so it joins a shard already running its test
file. The plan (`mutationShardPlan`) and each shard's own deal
(`timedShard`) read the same costs, of the same corpus listing
(`corpusCaseNames`); `testsAsItIsRuns` counts the same keys, so the
estimate and the deal can't drift. Held by `test/shards.test.ts`, "a
shard's tests as the compiler is are dealt with its mutations"; mutations
`deal-control-runs-unread` and `deal-control-runs-counted-twice`, and the
moved `tests-as-it-is-corpus-cases-uncounted` and
`tests-as-it-is-modes-merged`.

## Amendment: each shard's times are merged at the plan's speed (2026-11-22, #2304 (b))

A run's mutation times are merged after it, the newest of each mutation
over the plan's, the nightly's as each check's, for the next plan to deal
by. Nightly 44963035271 was dealt evenly, 6,710 to 6,744 s a shard, but its
shards 1, 13, 14, 15 and 16 took 1.22 to 1.32 times the plan's time of each
of their mutations at the median, and its others 0.89 to 1.01: a whole
shard alike, on one image, shards 1 and 2 started together, so GitHub's
machines, not the mutations. Merged as they ran, a mutation's newest time
was its machine's as much as its own, and the next deal took it so. **Each
shard's times are now divided by its machine's speed before the merge**
(`normalizedShardTimes`): the median of its mutations' times over the
plan's, of those the plan timed at over half a second, at least 20, held to
0.5–2, each shard read from its own artifact's folder (`timingsByShard`).
A mutation that grew against its shard's others keeps what it grew by. A
slow machine still makes its shard longer; the deal can't know which
machine a shard gets. Held by `test/shards.test.ts`, "each shard's times
are merged at the plan's speed, its machine's taken out"; mutations
`shard-speed-kept-in-times`, `shard-speed-of-a-few` and
`shard-times-one-folder`.

## Amendment: a site is placed where its own lines didn't change, under the path its file had (2026-11-23, #2337, the lead's decision)

The snapshot judge called a site unplaceable wherever its file had changed
since the map's commit, and git's diff, `--no-renames`, called every moved
file changed. So each of #2299's cut's tidy mutations was a survivor until
the map was recorded again: nine on the cut's branch, and on `main`'s check,
which raced the push's own map. Most of the cut's files moved 95 to 100%
alike, with an import's path or a visibility edited.

The judge, and the selection that reads the same diff, now ask the map in
its own terms:

- **A move is a rename.** The diff is `-M` (`changesDiff`, the one command
  of both), and a renamed file's change is the map's path with the edit's
  hunks in its lines there. A pure move selects nothing, and an edited one
  selects what its edit would have selected where it was.
- **A site is placed where its own lines didn't change.** `mapLines` moves
  each line by the hunks before it, to its line at the map's commit, and the
  map is asked there. That holds for a file edited in place too.
- **Never a guess.** A site is `all`, with its reason, in each of these
  cases:
  - a hunk touches its lines, a deletion inside them included;
  - its file is new;
  - its file moved with git's similarity below `placedSimilarity`, 90%.

This is the premise the judge already holds across files, not a new one.
An edit elsewhere can change which cases reach a site, and an edit in
another file was always accepted as leaving the map's answer standing. An
edit to the same file's other lines is the same kind of change. The
backstop is the same too: CI's runs of every mutation, the nightly's whole
map, and its drift check.

## Amendment: a mutation that needs the WASM is judged in the wasm job (#2346, 2026-11-23)

A mutation whose tests read rust-js.wasm, the playground's tests, read as
survived wherever the wasm is absent, as they skip without it. That's
locally, where building it needs a rust-lang/rust checkout, and in the
mutation shards, which don't build it. A source mutation's workspace
copies only what git tracks, so even a machine with the wasm didn't hand
it to the mutant.

Such a mutation says so, `needs: "wasm"`. The runner keeps one selection
in two modes:

- **Without `--wasm`**, everywhere else, it leaves them unrun, each said to
  be "judged in the wasm job" in its results.
- **With `--wasm`**, it judges them alone. It requires the wasm and sets
  `RUST_JS_REQUIRE_WASM`, so a test that reads it fails rather than skips.
  Each mutant's workspace links what the wasm build made, the wasm and its
  sysroot (`wasmBuild`, wasm/web/site.ts).

CI's `wasm` job runs `--wasm` after its build, with the shards' own
arguments. `plan`'s `wasm_mutations` starts the job where the selection has
one. The Nightly judges every one in its suite part's call, whose wasm job
runs anyway: its mutations part's would hold one more runner beside its
shards, past its budget (test/runners.test.ts). Its results, `mutation-results-wasm`, sit beside the shards': the next
check retries what the shards left to it only where it didn't catch it, of
the same commit (`resultsBase`). The first is `wasi-crate-files-emptied`.

Its Rust half, a mutation of `cfg(target_os = "wasi")` code, which needs the
wasm rebuilt per mutant, is left until such a mutation exists. A mutant of
a module run as a script, whose top level throws before the load mark
appended at its end, reads as not executed; a mutation there is placed where
a test imports the module instead (#2346's notes).

## Amendment: a runtime site's case forces the form that reaches it (#1979)

A runtime helper's mutation survived the whole corpus where the corpus
reached its site hundreds of times: of #1979's survivors, the runtime
`enumerate`'s three. The cases their mutations named had moved to std's
compiled `Enumerate`, and those that still reached the helper's site could
not tell its mutant apart. The runtime steps a chain only where it's made in
place (ADR 0456), and a crate's own iterator is made in place only where its
own `next_back` is called elsewhere in the program; so a case that holds a
runtime site forces that form, and is checked against the mutant, not only
against native Rust (ca1a3508e: a `size_hint` that doesn't shrink, an own
`count` that prints, a closure that drops beside a `take()`'s leftovers).

Whether a runtime site is reached is asked of the runtime itself, a marker
written into `std/runtime.js` at the site that appends its name to a file as
it runs, over the whole evidence (check:fast --all, the nets, React's port,
the crate corpus). A marker is a statement of its own or a comma expression
inside the one it marks: placed between an arrow's `=>` and the expression
that is its body, it became the body, and every enumerate threw before it
ran, so an evidence run called reached sites unreached. Each marked runtime
passes `node --check` and runs a case that reaches each site before the
evidence counts.

What's made in place is decided of each function body: a trigger in
`main` makes no chain in a `catch_unwind` closure or another function in
place. A probe that doesn't reach a site is no evidence that no program
does: `step-by-fold-unheld` looked unreachable from probes whose folds sat
in other bodies, and was reached, and caught, with the trigger beside the
fold. A site is called unreached only by the marked whole evidence.

## Amendment: a run says where it is while it runs (2026-11-24, #2418)

One `--changed` run of 247 mutations said nothing for 50 minutes and was
stopped by its timeout; the same mutations by name, 30 at a time, took 6.
A run said each answer only at its end, and a wait on a lock, which
sleeps the process, once. Now a run says each answer on stderr as it's
given (`Answers`, "answered k of n"), each phase as it ends, and every
minute where it is (`heartbeat`); `acquire` says its wait every minute,
with whose process holds the lock. Its stdout, CI's answers, is as it
was. Measured here with them, `--changed=7e3d83e73`, 401 mutations, in
one run: 58 s of control, 6 min 23 s of mutants, 7.5 min in all, the
answers steady at about 70 a minute. So one run costs no more than its
parts, and isn't split: what held the 50 minutes was a wait, which a run
now says. Tests mutations "a waiting mutation run says where it is, and
each answer as it's given" and leases' lock test; mutations
`answers-said-at-the-end-alone`, `heartbeat-silent` and
`lock-wait-said-once`.

## Amendment: a base's run reads the run's test map, and a mutation's error is its own (2026-11-24, #2341)

In six of the eight mutation shards of check 45043213276, #2299's cut,
the base run called every survivor the push's, and the check turned red
with nothing new in it. The cause showed in the shards' own logs. CI's
shards read the test map of 2cd281abf, a commit inside the push's batch,
after its base 66a035ec4. A map is read only of HEAD or an ancestor, so
the base's run, checked out at 66a035ec4, found none. A readability
owner's mutation that survived its tests there went to the snapshot
judge. Without a map in CI, that judge threw. The throw ended the base's
whole run, with no `results.json`, so each survivor of the shard was the
push's. The fit is shard for shard:
- each of the six that misfired had a surviving `tidy` mutation in its
  rerun;
- shard 6 had none, and marked 9 latent;
- shard 5's one `tidy` survivor was caught by its tests at the base, and
  the shard marked 4 latent.
Locally it worked because the machine keeps, or records, a map of the
base or an ancestor of it.

Two changes:
- **One map judges push and base.** A run's survivors run again at the
  base named the commit of the map the run read, `RUST_JS_TEST_MAP_OF`
  (`namedMapEnv`, set by `resultsAt`). The base's `headMap` reads that
  map (`mapOf`) of any commit the checkout has, whole and of this layout
  and toolchain. `mapLines` places it at the base by the diff from its
  commit, as it places an ancestor's at HEAD, and says so where a site's
  own lines changed. A run that names no map reads HEAD's or its
  nearest ancestor's, as before. It's an environment variable, not a
  flag, because the base's runner is the base's own code: an older one
  ignores it where it would refuse a flag it doesn't know.
- **A mutation's error is its own verdict.** Where no map places a
  site, the snapshot judge says so as an inconclusive verdict
  (`unplaced`), never a throw and never a survivor, which a base's run
  would call latent. Any other error thrown while judging one mutation
  is that mutation's inconclusive verdict (`judgedApart`), so the run's
  other verdicts stand. An inconclusive mutation fails every run, as it
  did (`failingOf`), and the run's last line and step summary now count
  them ("N inconclusive", `inconclusiveSaid`). A runner fault that
  leaves many inconclusive results shows as one, not as survivors.

Tests:
- test/survivors.test.ts' "a survivor's run at the base is judged by the
  test map the run read" (a runner at the base that writes the map it
  was named);
- test/test-map.test.ts' "a worktree reads the map …": a named map of a
  commit after the checkout's; none of a commit the checkout lacks or of
  another version;
- test/mutations.test.ts' "a mutation's judging that throws is its own
  inconclusive verdict …".

Mutations: `base-run-unnamed-its-map`, `named-map-unread`,
`named-map-of-a-commit-unhad`, `named-map-of-another-version`,
`unmapped-taken-for-a-survivor`, `unmapped-judged-a-survivor`,
`judging-throw-ends-the-run` and `inconclusive-uncounted`.

## Amendment: a retirement checks React's port at its pin first (2026-11-24, #2525)

A retirement's controls ran the compiler's unit tests first, then built
the compiler and regenerated React's port at its pin. Where the port
doesn't regenerate as its pin committed, the whole run is refused, and
that's the control a compiler change most often fails: a run spent 6 min
on the unit tests and the build, then refused on the pin. Now the order
is `retireControls`': the compiler's build, the port's control, then the
unit tests and the crate corpus. A pin to fix is said after the build and
one regen.

The regen also says its time as it ends, `port regen <s> (usual <s>)`,
and keeps it with the checkout's steps' times (scripts/timing.ts),
whether it's run alone, by a retirement or by a push. Before, only a
regen run alone wrote its time, to the machine's log.

Mutation `retire-port-control-late`.

## Amendment: a retirement's controls run at once and are kept for their tree (2026-11-25, #2485)

After the port's control, the compiler's unit tests and the crate corpus
ran one after the other: 103–116 s and 89–98 s on the M5 Pro
(`retirePhases`, 2026-11-24's retirements), and again for every
retirement of the same tree, though nothing they read had changed. They
build and run nothing they share, so they now run at once
(`retireControls`); the port's still goes first, alone, so a pin to fix
refuses the run after one regen (#2525).

Their verdicts, the unit tests that fail as the crate is and the crate
corpus's rows, are kept in `target/mutants/retire-controls.json` with the
tree they passed on (`keptControls`; scripts/judged.ts' `treeOf`, the
files a mutant's workspace is made of). Each control is of that tree alone,
its compiler, its pin and its rows, so a second retirement of it builds the
compiler and asks none of them again. Another tree's are none.

## Amendment: a survivor's run at the base is in a worktree of its own (2026-11-25, #2578)

A survivor's run at the base (`resultsAt`, scripts/latent-survivors.ts)
checked the base out in the run's own checkout, and checked it back after,
to build its crate where the run's was (#2222). That checkout is the one
under test. check:fast's job of the change's mutations runs beside its
other jobs, and the push's `judged` phase runs before the push compares
the tree it checked. So:

- for about 80 s, the other jobs tested the base's files with the
  change's compiler: on the Mac mini 13 and 5 files failed in 0–3 s
  (#2438), and a check can pass as wrongly;
- a push whose change had a latent survivor was refused, its tree changed
  while it was checked, and couldn't land at all.

Now the base's runner runs in a worktree made at the base, under the run's
mutants' store (`baseTree`), which the run's lock keeps from the prune,
with its own packages and target. Its results are read where it wrote
them, so the run's own `results.json` and `timings.json` are never written
over. The worktree is removed however the run ends, and one a killed run
left is removed as the next begins. The checkout's own changes no longer
stop it, as nothing of the checkout moves.

Its cost: the base's crate is built whole, 283 to 320 s of a CI shard
(#2222's measure), where the checkout's build was reused. It's paid only
where a change's mutations have a survivor. Correctness first.

What holds it: test/survivors.test.ts, "a survivor's run at the base is
the base's runner's, in a worktree of its own, and leaves the run's
checkout as it is throughout". Its base's runner says the run's
checkout's HEAD and files as it ran. Mutations `base-run-checks-out-the-checkout`,
`base-run-by-the-checkouts-runner`, `base-run-leaves-its-results` and
`base-run-leaves-its-worktree`. And "no script checks out, switches,
stashes or resets the checkout it runs in", which names the scripts that
move a checkout of their own (#2578's audit).

## Amendment: a retirement's fail-fast stop ends its suite at once (2026-11-25)

A retirement's suite runs check:fast `--fail-fast` (#2256): at its first
failure, the pool stops the jobs running. It stopped each job's process
alone, with SIGTERM, and then read its pipes to their end. What a job had
started, a test's cargo or compiles, held those pipes, so the pool waited
on them. Retire-B's `filter-meaning-unmade` (2026-11-24) failed
test/compiled-std.test.ts 2 min into its suite, very likely a catch, and
its suite ended at its 2,400 s limit. The verdict said it said nothing,
and the name had to run again.

Each of the pool's jobs now runs as the leader of a group of its own,
which goes with check:fast (`guarded`). A stop, fail-fast's or a job's
timeout, tells the group to end, kills it `stopGrace` (5 s) on, and lets
go of its pipes then, where a process the job started in a group of its
own still holds them (`stopJob`). The run ends within seconds of its
first failure, and the retirement reads its catch.

What holds it: test/timing.test.ts, "check:fast's fail-fast stop ends the
run within seconds, though what a job started holds its output": a job
whose own group holds its pipes, and one whose child, in a group of its
own, does. Mutations `fail-fast-stops-the-job-alone`,
`fail-fast-waits-on-held-pipes` and `pool-job-in-the-checks-group`.

## Amendment: every stale find said at once (2026-11-25, #2586)

A code move, a split or a relay leaves mutations' finds stale, and
test/mutations.test.ts' "every mutation applies" named the first alone:
the worker-03 agent's eight runs of it in one day, a stale find each, and six
of the worker-04 agent's pushes refused in their fast phase. `bun
scripts/stale-finds.ts [names..]` names every one whose `find` doesn't
apply once, each with its list and its find beside its file's nearest
text now (`nearest`: the window of its lines holding most of them, each
where the find has it counting twice; of a one-line find, the line sharing
most of its words), and the test names them all. Test: "every stale find
is said at once, beside its file's nearest text"; mutations
`stale-finds-first-alone`, `stale-finds-near-by-membership-alone`.

## Amendment: a run again fits one argument, and a source mutant's error is its own (2026-11-28, #2617)

The nightlies of 11-26 and 11-28 lost shards 2, 6, 7, 11 and 16 of 16
part-way, hundreds of mutations in, to `E2BIG` spawning a mutant's tests
run again (`judgeTwice`). The run again was a `-t` of every failed test's
name (`narrowedTo`), and a source mutant that failed some 600 of a file's
long-named tests made a pattern past 128 KiB, the longest one argument
Linux spawns (`MAX_ARG_STRLEN`); macOS bounds only all of them together,
so a Mac never saw it. Two rules end it:

- **A run again's pattern fits one argument.** `longestArgument`
  (test/child.ts) is Linux's bound, and a pattern as long or longer runs
  the mutation's tests whole, as a setup's failure already does. Which
  tests fail twice is judged as before.
- **A source mutant's error is its own verdict.** The 11-24 amendment's
  `judgedApart` held for a compiler mutant alone: a source mutant's
  judging ran outside it, so its throw ended the shard. Its judging is
  `judgedSource`, `sourceMutation` under `judgedApart`.

Tests: test/mutations.test.ts' "a mutant is caught by a test that fails
with it twice …" (a pattern a name short of the bound, and one a name
past it); test/source-mutations.test.ts' "a source mutant whose judging
throws is its own inconclusive verdict …"; test/child.test.ts' "the
longest one argument a spawn takes is longestArgument's …", which holds
the bound to Linux's on CI. Mutations `narrowed-pattern-past-one-argument`
and `source-judging-not-apart`.
