# 1182. A case's builds compile in one rustc session

Status: Accepted (2026-11-22, the lead's design review of #2301; Tu chose to
speed up the loop beside the crate split).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is how the
tests' harness compiles, not anything the JS does.

## Context

[#2296's measurement](../dev-loop.md#where-a-checks-time-goes-2296-2026-11-22)
split a warm corpus run on the 12-core M6. Compiles are 88% of it, and each
case compiles twice: as a program, and as a library a crate uses (ADR 0360).
The two compiles share nothing. Each pays the process's start (about 38 ms,
31% of a compile, mostly loading rustc's libraries) and rustc's front end,
from parsing through borrowck (15%). Yet their rustc arguments are the same:
the library build only adds `--library --manifest`.

## Decision

**The worker, `rust-js --worker`, compiles a job's builds in one rustc
session. It is the test harness's way and dev-only.** Every other host
compiles one build per process, as `rust-js <input.rs> ..`.

- **The protocol.** A job is one line of JSON on stdin:
  `{"builds": [[<a build's arguments>], ..]}`. Each build is the argument
  list of its own compile, parsed by the same `command::parse`, so there's
  no second schema of options. The answer is one line on stdout, with one
  entry per build: `{"code", "stdout", "stderr"}`, what its own process
  would have given; or `{"apart": "<why>"}`, a build the harness then
  compiles alone. Jobs are answered in turn until stdin ends.
- **Which builds share a session.** The builds must differ only in what
  they write: `-o`, `--library`, `--manifest` and the compile cache's
  record (`--emit=dep-info`). No build may take any other rust-js flag or
  write metadata. The worker refuses any other job (`worker.rs`'s
  `shared_part`).
- **What they share.** One session runs rustc's front end once. The
  crate's THIR and MIR are copied once, before borrowck steals them
  (`lower::copied_bodies`), and each build filters them itself
  (`lower::bodies_of_build`, the library leaving out what only an unlowered
  impl reaches, ADR 0425). Each build lowers in a lowering session of its
  own (ADR 1111). Builds may come in either order.
- **Silent so far, or apart.** rustc answers a query it has already
  answered without replaying that query's diagnostics. Some lints' order
  also depends on when MIR is built. So a build after one that printed
  could print less, or in another order, than its own process. The session
  therefore goes on to the next build only where nothing has printed so
  far (no diagnostic, no warning, no replay of props, ADR 0213). The
  remaining builds answer `apart`. The last build the session ran gets
  rustc's own end of session, as its process would.
- **Output per build.** The worker redirects the process's stdout and
  stderr to a file for each segment: the front end and the first build's
  lowering, each later build's lowering, and each build's publish. A
  build's answer is its own segments, in order.
- **Publishing.** Each build is published once rustc is done, in job
  order, by the same `finish_build` as a compile alone. Each build's exit is its
  own, so a library's refusal can't reach the program's files.
- **The cache's record.** rustc writes one record of what the session
  read, after every build has lowered. Each build's record is a copy of it,
  so it names what every build read: a superset of the build's own, which
  can only make the cache miss, never give a stale compile.
- **Crashes and deadlines.** A crash outside rustc, or a job past its
  deadline, gives no answer, and the harness compiles each of that job's
  builds alone.
- **What a build leaves behind.** A build leaves its statics to the next
  one. Each `static` and `thread_local!` of the compiler is classified in
  `test/architecture-statics.test.ts`: a constant, one compile's own, set
  by a flag the worker won't share, a flag's report, the process's
  decisions' counts, or a name's identity (compared only for equality). A
  new static must say which class it is.
- **Session options.** The session sets `always_encode_mir` when any build
  is a library, as the library's own compile does. That option only
  changes the crate's hash and the metadata. The program build never reads
  the local crate's hash (only a library's contract does), and the worker
  writes no metadata.
- **The harness.** `test/child.ts`'s `runBuilds` compiles a case's builds
  through the compile cache (`test/compiles.ts`'s `keptBuilds`). Each build
  is found under its own compile's key, exactly as before. Those not found
  become one worker job, recorded and kept as their own compiles would be.
  A compile alone is `keptBuilds` of one build. Today the corpus is the
  only caller (`test/corpus.test.ts`, the cases that run); one worker
  process starts per job.

## Proof

- `test/worker.test.ts` compiles a sample of the corpus (every
  `library-refused` case among it) three ways: each build in a process of
  its own, then one job program-first, then one job library-first. It
  compares every build's exit, stdout and stderr, and every file written,
  byte for byte, and counts the builds run apart. The nightly runs it on
  every case (`RUST_JS_WORKER_CASES=all`, in parts).
- The whole corpus passes through the worker.
- Mutations: continuing after a build that printed; sharing a session of
  builds with other flags; losing a later build's own segment.

## Consequences

- A warm corpus run pays one process start and one front end per case,
  not two. On the quiet 12-core M6, on one compiler build: 68 s → 48 s of
  wall time; compiles summed 723 s → 490 s (docs/dev-loop.md). No job fell
  back to compiling apart.
- Step 2 (#2301) reuses one worker across jobs, so the start is paid once
  per test process. The statics catalogue is its precondition, and it is
  already held. Every test's `compileJs` moves onto the worker only after
  the nightly's whole-corpus check has run clean for a few nights (the
  lead, 2026-11-22).

## Amendment (2026-11-22, #2301 step 2): the worker is kept between jobs

**A test process keeps one worker and sends it every job.** Before, it
started one worker per job. Now it starts the worker at its first job
(`test/workers.ts`'s `workerJob`), so the compiler's start, loading rustc's
libraries, is paid once per test process, not once per job.

- **Jobs:** sent one at a time, in the order they're asked. A job that
  gives no answer by its deadline (each build's `timeout`, summed), or
  ends the process, stops the worker. That job's builds are compiled
  apart, and the next job starts a new worker. A job of another compiler,
  working directory or environment also starts its own.
- **Process lifetime:** the worker neither keeps the test process alive
  nor outlives it. Its stdin closes as the test process ends, and
  `guarded`'s watcher stops it.
- **Cache:** `test/compiles.ts`'s `keeping` is the cache's one part of a
  case's builds (plan, found, record, keep), which `keptBuilds` and
  `keptBuildsAsync` share. A compile alone is still a job of one build.
- **What each build leaves:** each rustc session sets its own state as it
  starts: jsx's crate id in `expand`, its props' state in `begin`, its held
  diagnostics as its emitter is made, and the lowering session. Every other
  static is classified in test/architecture-statics.test.ts. So a job after
  any number of others compiles as its own process would.
  test/worker.test.ts's rows all go to one kept worker (checked by its pid),
  each case's builds after every earlier case's, in both orders.
- **Memory:** a kept worker holds 150–240 MB between jobs, steady over a
  corpus part's ~240 jobs. A large case's peak (up to 1.7 GB) is its own,
  as it would be in a process of its own.
- **Measured** on the quiet 12-core M6, warm, cache bypassed, against step
  1: corpus wall time 48 s → 37 s, compiles summed 490 s → 374 s, process
  start and exit 118 s → 16 s. Against the two-process harness before
  #2301: 68 s → 37 s (−46%).
- **Mutations:** `worker-not-kept`, `worker-answers-swapped`.
