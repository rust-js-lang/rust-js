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
