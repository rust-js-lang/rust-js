# 1124. Decision coverage: which answers the nets' programs reach

Status: Accepted.

Case: none of [0262](0262-when-rust-and-js-disagree.md)'s: what's counted
while the nets run, no change to what any program does.

## Context

The nets find wrong answers where their programs reach them. Of the 38
P0s closed in the two days to 2026-11-12, audits found 15, the
metamorphic net 12, and generated programs 3. #1349, an overlapping
aggregate `ptr::swap` that changed values in both builds, wasn't found by
more seeds of the memory net, but by one interaction its generator
couldn't write, added by hand. The oracles were good enough; what no
program reached went unseen.

A decision of the compiler's, one owner's answer, is where two lowerings
part: a `RefCell`'s borrow counted, momentary or unchecked; a `&mut`'s
form; an `Rc` counted or not; how std collects a `Vec`. An answer no
program of any net gives is untested, however many programs run.

## Decision

**Each decision's owner counts its answers where it gives them, and
`bun run decision-coverage` runs the nets and reports which answers no
program reaches** (`src/decisions.rs`, `scripts/decision-coverage.ts`).

- **One catalogue.** `decisions.rs`' `CATALOGUE` lists each decision, its
  owner and every answer it gives. A record of a name or answer it doesn't
  list is refused as the compile runs, and `test/decisions.test.ts` holds
  each catalogued decision to a record in its owner and each record's
  literal answers to the catalogue.
- **Off unless asked.** `RUST_JS_DECISIONS` names a file. Unset, a record
  is a test of a flag read once. Set, each compile appends a line of every
  decision's every answer, its count, `0` of one it never gave, its build
  (`std` of `--std-iterators`, or `plain`) and its shape (`library` of
  `--library`, or `program`). So "never reached" is a sum of zeros, not a
  name missing. The driver writes the file, as only it writes files.
- **The run.** Each net runs with a file of its own, the compile cache off
  (a compile given again counts nothing): the corpus in parts, built as
  libraries too; iterator forms; generated programs; the memory, consumer,
  std and twin nets; the tables; the metamorphic net; the neighbour
  shuffle; and the crate pairs. The report,
  `target/decision-coverage/report.md`, lists the answers never reached,
  those reached in one build or one shape alone, those one net alone
  reaches, and every answer's count by net.
- **What a gap becomes.** A never-reached answer, ranked by how likely it
  is to be wrong, becomes a generator template or a corpus case, run
  against native Rust. A wrong answer found so is a P0. Answers one net
  alone reaches say which net a retirement would cost.
- **Where it runs.** The nightly, not each push: it runs every net in
  full, so it counts as it runs, the extra a compile some 1,700 records
  and a file's append, no run of its own. `nightly.yml` sets
  `RUST_JS_DECISIONS` of each job, and of the suite (`check.yml`'s
  `test`, by its `nightly`) and the generated programs
  (`rustc-tests.yml`'s `fuzz`, by its `decisions`); each job sums its
  file, a line a compile, to one of some 50 (`--sum`, the
  `decision-counts` action), and the `decision-coverage` job merges them,
  each job a net (`--merge <dir>`), into the run's summary and an
  artifact. An empty setting, a workflow's of a run not the nightly, asks
  none. Run here, `bun run decision-coverage` takes some 3 hours of the
  14-core Mac, each net as the nightly runs all of it, the iterator matrix
  its `pairs` sample, so it's run of the nets a change concerns, named.

The first catalogue is 17 decisions of 49 answers: `RefCell` borrows and
`try_borrow`s, `&mut` forms (ADR 1086), generic code's forms (ADR 1115),
writers' options (ADR 0137), dictionaries' routes, given type facts,
`Option` payloads, iterators given as they are, `Rc`s counted, collects in
place and `IVecCollect`'s ways, copies, what a type drops, a `RefCell`
field holding its value, and stored places. A decision joins by its owner
recording it and a catalogue entry.

## Consequences

- A record is in its owner, beside the answer it counts, so a refactor
  that moves the answer moves the record with it.
- Counting adds no cost to a compile not asked, and none to any output.
- A decision asked many times a compile, `DropQuery::drops`, counts each
  time; the report says reached or not, and by which nets, not how often
  a program asks.
- Coverage of answers is not coverage of combinations: an answer reached
  by one program may still be wrong beside another's. The report's
  one-net and one-shape lists point at those; the generators' isolation
  and build-shape variation are the next steps of the testing plan.
- The first run here (2026-11-11), every net but the neighbour shuffle,
  the object model's and the crate pairs, the metamorphic net two-thirds
  through: 48 of the 49 answers reached; `writer-options`' `shown`, a
  writer of a type a library's function shows with its options (#1331),
  none; `refcell-try-borrow`'s `unchecked` and
  `library-writer-options`' `plain` the metamorphic net's alone, of
  programs alone.

## Amendment: the second pass, finer answers (#1376)

The first run reached 48 of the 49 answers: a catalogue that coarse shows
few gaps. So each answer is now counted apart by where it was given and,
of a decision that says the type it answers of, by that type's kind; and
the decisions the first pass left, because their facts are crate-wide
sets, are counted where they're answered.

- **Bodies.** Each answer is counted of a body of the crate's own,
  `own`; of another crate's the build lowers, a library's or std's,
  `other` (`decisions::in_body`, set as `lowered_from_mir` lowers a
  body); or of neither, the crate's analysis or what's lowered outside a
  MIR body, `crate`.
- **Kinds.** A typed decision (`typed: true`, `record_typed`) counts its
  answers by the type's kind: a number, an array, a tuple, a struct, an
  enum, a type parameter, a reference, or another, as `recognition::kind_of`
  says, `decisions.rs` listing the kinds. `copy-if-needed`,
  `type-drops`, `option-payload`, `mut-ref-form` and `back-stepped` are.
- **New decisions.** `back-stepped`, whether a value is made knowing its
  back, by the crate's types, a generic function's own, a made local, or
  not (`iterators/back_facts.rs`' `stepped_from_its_back`), its
  `back-counted` and `stepped-local` beside it; `hint-seen`, whether a
  collect or an extend asks its iterator's hint where the program sees it
  (`iterators/hint_asks.rs`' `collect_hint_seen`); and `extend-reserves`.
  22 decisions, 64 answers.
- **The file.** A line is `decision answer kind body count build shape
  owner`. Only the first compile into a file writes every answer, `0` of
  each never given, so the report knows each there is; each after writes
  what it gave, where every line of every kind and body each compile
  would grow a net's file past a gigabyte.
- **The report** says, beside the answers never reached, each answer
  never given of a body or a kind another answer of the same decision
  was: one its owner is asked of, so a gap, where a body none of its
  answers is given of, the crate's analysis in a function's, isn't.

Of the first gaps: `writer-options`' `shown` is the crate pairs', 23
times, uncounted in the first run; `refcell-try-borrow`'s `unchecked` is
corpus case try_borrow_of_a_cell_never_contested's, as a program and as
a library; and `library-writer-options`' `plain` the crate pair
library_writer_without_options', an app's padding writer given the
library's plain ones, each against native Rust.

## Amendment: an answer a local holds is checked before the nightly

`generic-form`'s `tagged`, a write's result given to generic code
(#1410's `Form::Tagged`), was recorded through a local, `let answer =
match form { .. }`, and never catalogued. `test/decisions.test.ts`'s
"each record names a catalogued decision and its answers" read only a
record call's own literals, and only the nightly counts, so every compile
it counted that reached one crashed on `count`'s assertion: 33 of nightly
43133510260's 117 jobs, the corpus and every net among them. The test now
reads, of an answer a local holds, each literal the `let` before the
record gives, an argument of a call (`unreachable!("..")`'s) aside; and
"a compile asked counts generic code's tagged form of a write's result"
compiles corpus case write_result_given_to_generic_code counting, so the
answer is given, counted and catalogued in the per-push check. `form_of`
never asks `Form::Valued`, so its arm is `unreachable!`, not an answer
the catalogue lists that no compile gives. 22 decisions, 65 answers.
