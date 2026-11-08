# Finding wrong answers

A wrong answer hides where nothing compares the JS with native Rust. This
page lists the techniques that have found wrong answers, what each found,
and the lessons behind them, so the next net is built on what worked.
[TESTS.md](../TESTS.md) is the authority on which suites exist and what
they cost; [bug-kinds.md](bug-kinds.md) on how a bug is classified once
found. This page is about **how bugs are found**.

When a new technique finds a wrong answer, or a wrong answer is found that
none of these would have, add it here in the same commit as the fix.

## The techniques

| Technique | Where | What it found | Why it works |
|---|---|---|---|
| A corpus case per behaviour, run against native in both builds | `test/corpus/`, `test/corpus.test.ts` | Most fixes' first proof | Each case pins one behaviour against native, not against a snapshot |
| Generated tables of every combination | The iterator matrix (107k cases) and conformance table (21k), ADR 1090/1092, `test/iterator-matrix.test.ts` | Most of the iterator P0s since 2026-11-04; 23 of the last 30 iterators × protocol gap issues | Combinations nobody would write by hand: source × adapters × consumer |
| Panic points | `ITERATOR_TABLE=points` (#817): every closure call, item drop and source drop is the k-th event, with one panic per run | #843–#846 on its first run | Unwind paths are rarely written by hand and are where drop orders differ |
| Panic points beyond iterators | `ITERATOR_TABLE=unwind` (#854, `test/unwind-points.ts`): 16 families of operations × 11 shapes of value, each make, drop, clone and comparison of a `D` and each closure call the k-th event, by #817's runner and oracle | 16 P0s on its first run (#897–#912), 591 of 3,470 cases: partial clones, fill values and the rest of an `extend` leaked; a map's `clear` and a `BinaryHeap`'s and the unstable sorts' holes dropping twice; derived `Default`s and combinators' `self` leaked; and five with no panic at all, `HashMap::retain`'s drop of a handle, `VecDeque::retain_mut`'s eager drops, `Rc`'s `==` by pointer, and the order a stable sort, a `BTreeMap` and `max` call the crate's `Ord` | A comparison that prints is a point too: what calls the crate's `Ord`, `Eq` or `Clone`, and in what order, is as observable as a drop |
| A fixed table row grown by hand: more items, all equal | Beside a fix of the unwind table's rows, a probe of the same operation with three entries that each print alike (`D(1)` thrice), so an unspecified walk order can't change the lines | #920 (hashbrown stops at a panicking drop, the table's one-entry `HashMap` rows can't show it), #921, #922, while fixing #899–#919 | A table's shapes are small; what happens to the rest after the first panic needs a rest, and equal items keep a hash map's order out of the answer |
| A full run, not a sample | `ITERATOR_MATRIX=all`, `bun run nightly` | #823, #834, #835, #833: fix-on-fix regressions the sampled slices passed | A fix that makes a shape compile exposes every path through it |
| Rebatching: the same rows in other company | #818's `pairs` and rows a change names | #850, wrong only beside another function using the same type | Crate-wide and per-type facts leak between unrelated functions |
| A context grid: one function makes a type-wide fact, another uses each adapter over that type | Hand-generated programs (#851's probe): a chain folded from its back in one function, each of 13 adapters over the same chain stepped back four ways in another, against native in both builds and on `main` | #859, an `inspect(f)` made JS's `.map(f)` beneath a `$mapEnds`, which threw; and #850's and #856's shape again | A fact keyed by a type is set by one value and read of another; the grid puts every adapter's maker beside the one that sets it |
| The same input compiled again | `test/determinism.test.ts`: each corpus case compiled twice in separate processes, as a program and as a library, its JS, map and manifest compared byte for byte (`DETERMINISM=corpus`, the nightly's); rustc's `rustc::potential_query_instability` lint to find each walk of a std map | #866: ranges' fixpoint walked its callees in a `HashSet`'s order, so a `u32`'s `n + 1` wrapped in 5 compiles of 8. The fix's audit found 519 such walks, which ADR 1098 made deterministic by construction | A snapshot that fails at random is a decision that depends on the order a map is walked in: a fixpoint that widens after so many changes, a first-come name. Each compile is a fresh draw, so compile again and compare |
| A second build as an oracle | `--std-iterators` (std's own structs) beside the default build | #742, #753, #770, #772, #813, #839 | Two lowerings of the same program must agree with native |
| Generated drop-order sweeps | Hundreds of small programs per change (e.g. 326 `step_by` programs, #827) | #827's reversed slice drop, #810 | Drop order is invisible unless every drop prints |
| Mutations | `scripts/mutations/`, ADR 0093 | Dead code and untested rules; a surviving mutation is where to look | A rule whose removal changes nothing has no test, or is dead |
| Removing code only with the full rows of what it decides | DEVELOPMENT.md "Landing" | #823 (a rule removed on the corpus and slices alone) | "Equivalent" judged on a sample isn't equivalent |
| Real crates | `bun scripts/crate-corpus.ts` (22 crates) | #737, #794, #795, #828 | Real code combines features in ways the corpus doesn't |
| The ports | react-in-rust, alien-signals | Most early representation and readability gaps | A whole program, compared file by file with its original |
| Replacing a hand-written rule with rustc's own, and reading every difference | #887: drop glue lowered from rustc's `drop_glue` shim (`rustc_mir_transform::build_drop_shim`), then printed beside the hand-written walk for each class of type | #893: an enum with its own `Drop` whose variant's later fields were dropped before the abort | rustc's MIR is the native behaviour, so where the two lowerings differ, the hand-written one is the suspect. The shim's cleanup blocks show unwind paths no corpus case had covered |
| Probing against native before deciding | Any decision whose JS form is in doubt | #832's three wrong payloads; #809's unsound "pass the box on" | A plausible rule is checked before it's built |
| Mapping a rule's deciders, then probing every case of its model | Before a refactor of a scattered rule (#820): each destination × operation, with the crate's own observable step, `size_hint` that prints | #848 (a `Result`/`Option` collect's hint, through std's `GenericShunt`) and #849 (every `extend`), neither touched by any of the six P0s the rule had | The model's table is the test list: a case no decider handled is one nobody wrote |
| rustc's own tests and generated programs | `bun run test:rustc`, the nightly's 600 programs (ADR 0089, 0092) | Long-tail language cases | Programs written by others, or by no one |
| Reading accepted differences against ADR 0262 | Guide "Differences", ADR exceptions | #115, #847: documented "rare and harmless" differences that native showed were results changing | A difference written down as accepted is never tested again |

## The lessons

1. **Generate the space; don't sample it by hand.** Every large find
   came from a generated table: adapter × consumer × source, or every
   panic point. A hand-written case covers what someone thought of.
2. **Context is part of the input.** An answer that depends on another
   function in the crate, a per-type or crate-wide fact, is a bug class of
   its own (#823, #834, #850). Test the same rows in other company, and
   prefer facts per value over facts per type (#814, #851).
3. **A refusal is safe; a shape that newly compiles is where wrong answers
   start.** Run the full rows of whatever a fix makes compile (#818's
   gate), not the sample.
4. **Unwind paths are where orders differ.** Inject a panic at every
   point: closures, drops, the source's drop. A second panic during
   unwind aborts in Rust (#847), so a table's cases never panic of
   themselves (an `unwrap` of `None`), only at their point.
5. **Equivalence needs the full rows.** Before removing code whose
   mutation survived, run every row of every adapter it decides for.
6. **Every accepted difference is a claim to re-test.** ADR 0262 allows
   an exception only by the user's decision; an agent's "harmless" isn't
   one.
7. **Two oracles beat one.** Native is the oracle; a second lowering of
   the same program (`--std-iterators`) finds bugs one build hides.
8. **A program all refused passes all.** A table whose shared prelude
   rust-js refuses refuses every case, and a refusal passes. The harness
   fails a batch whose every case is refused (#854's first run of its
   keyed maps passed so, in 392 s).

## Nets still to build

| Net | Issue | What it would find |
|---|---|---|
| Neighbour shuffle: random groups of corpus cases compiled as one crate | #853 | Every crate-wide or per-type fact leaking between functions, in every area |
| Matrices for `MutRefForm` × operation and `NoneForm` × writer × reader | #855 | The two clusters with the most P0s and regressions after iterators |
| TypeScript callers of each `.d.ts` with `null`, `undefined` and absent keys | #822 | A `.d.ts` that promises more than the lowering trusts |
| Crate-corpus probes run, not only compiled | #840 | Real crates' behaviour against native |
| Metamorphic rewrites: each corpus case inlined, outlined, made generic, called through `dyn`, moved to a library | #939 | Answers that depend on the shape around a value, not on the value |
| The numbers grid: every integer and float operation × type × boundary value | #940 | Overflow, wrapping, casts, shifts and float edge cases |
| The formatting grid: every format spec × type | #941 | The most visible output a program has |
| Async panic points: drops and panics across `.await` | #942 | Drop order in futures' state machines, cancellation and joins |
