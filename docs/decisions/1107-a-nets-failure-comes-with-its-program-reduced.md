# 1107. A net's failure comes with its program reduced

Status: Accepted (#1088, `dev-loop`; the user's queue, 2026-11-09).

Case ([0262](0262-when-rust-and-js-disagree.md)): none decided here. This
is the development loop's: what a failure is isn't changed, only how small
the program it's reported with is.

## Context

Each net hands an agent a large program that goes wrong: a metamorphic
rewrite of a corpus case, 24 to 160 lines (ADR 1104), a neighbour group's
crate, a dozen cases in one (ADR 1103). Before anything's fixed, the agent
reduces it by hand to the few lines that still go wrong, compiling each try
natively and as JS. That is a large share of a P0's time, and none of it is
the fix.

## Decision

**A failing program is reduced automatically, C-Reduce style, and each
net's failure is reported with its program reduced.**

- **The reductions are the rewriter's** (`metamorph reduce`,
  `scripts/metamorph/src/reduce.rs`), largest first: an item removed, `main`
  kept, of a module, an impl and a trait too; a statement; a match's arm;
  an `if` made either branch; a format macro's argument with its `{}`; a
  number made `0` and a string `""`; a `let`'s type left to inference; an
  enum's variant with each arm naming it; a struct's field with each
  literal and pattern naming it; a free function called once inlined, a
  `let` of each argument, and removed. Each is made whatever it then means:
  the oracle decides.
- **The oracle** (`scripts/reduce.ts`): a reduction is kept where rustc
  still accepts the program, native Rust still runs it to its end, and
  rust-js still fails the same way: its same first error, or the JS ending
  otherwise than native Rust, a throw where it returns, the same way, or
  else the first line the two print apart, the same on each side. Or a
  looser oracle given, `differs` or `error:<text>`.
- **Greedy and in parallel:** the next reductions in their order, one job a
  core and no more than the run's memory budget holds (`memoryBudget`,
  #1084), each a test's, checked apart; the first kept,
  in their order, is the program, and the round starts again from it, till
  a round keeps none, or the budget's spent. A reduction is tried only
  where it's smaller: the text without its spacing, a digit but `0` a
  little more. The result is formatted by rustfmt.
- **Wired into the nets:** a failure of the metamorphic net or the
  neighbour shuffle that isn't on its known list is reported with its
  program reduced (`test/reduced.ts`), of the build it fails in, each part
  of a net run in parts given its share of the cores; a neighbour group as
  one file, its cases' text in their modules. `NET_REDUCE=0` gives the
  command instead; `NET_REDUCE_BUDGET` bounds each. A `library` rewrite,
  two crates, isn't reduced.

## Consequences

The four open failures of the metamorphic net's round 3 and #1002's
remaining neighbour pair reduce in 6 to 42 s each, to 9 to 32 lines from 24
to 128 (docs/dev-loop.md). What's left is what the reductions can't say:
an expression made simpler than a statement removed, a type made simpler,
a `library` rewrite's two crates.

Held by `test/reduce.test.ts`: a program rust-js refuses reduced to what
it refuses, a program that runs as native Rust does left as it is, and the
rewriter's reductions each smaller.

## Amendment: no step that's undefined behaviour is kept (2026-11-09, #1104)

Reducing #1099, semver's bytes written through `std::alloc::alloc`'s
pointer, kept a step that dropped a write and left its byte read: a read of
memory never written, undefined behaviour, whose native run prints what it
happens to. The repro still "failed the same way", and was no program.

**A step of a program with `unsafe` code is kept only where Miri finds no
undefined behaviour in it** (`failure`'s `undefined`, beside `rejected`).
Safe Rust can't be undefined behaviour, so a program without `unsafe` isn't
run under Miri; every raw pointer read, `alloc`, `MaybeUninit`'s
`assume_init` and `transmute` is `unsafe` to write. Miri is the one judge
of it, `test/miri.ts`, which the generated programs' `FUZZ_MIRI=1` asks too
(#1085). A program with `unsafe` code isn't reduced where Miri isn't
installed: the reducer says so, with how to install it, and the nets'
reports then give the command. One that's undefined behaviour itself isn't
reduced either. Miri's sysroot is asked for once, `MIRI_SYSROOT`, which the
checks run apart are given.

Held by `test/reduce.test.ts`: "a step that makes the program undefined
behaviour isn't kept, as Miri finds it is", #1099's program, whose step of
the rewriter's that drops its second write Miri finds undefined; and "a
program that's undefined behaviour isn't reduced". They run where Miri is
installed, and in the generated programs' nightly job, which installs it
(`RUST_JS_REQUIRE_MIRI=1`, which fails them where it isn't). Without the
check, each fails.

## Amended (2026-11-11, #1306): a reduction holds its share of the memory, and a part of a net says what it held

The nightly's neighbour shuffle was stopped by the runner's shutdown every
night from 2026-11-08 to 11, and its metamorphic net ended in a bare exit
143, no test failing in either. Each runs four parts at once on a 16 GB
runner, and each part reduces its failures one at a time. Run on the 16 GB
Mac with the day's seed, the shuffle's four parts held 5.4, 9.7, 9.7 and
7.2 GB at their peaks, nearly all of it a reduction's native variants, one
8.6 GB: a variant whose loop no longer ends allocates without end, which
Linux stops at 4 GiB a run (`RLIMIT_DATA`) and macOS doesn't, so four parts
reducing at once can hold 16 GB.

- A reduction's variants together hold at most `--memory` GB, the run's
  memory budget (`memoryBudget()`) unless it's given, and a part of a net
  in `n` parts gives its `n`th (`test/reduced.ts`). Each variant's check
  runs as at most that share, at least 1 GB, as many at once as the share
  allows, and at most `variantTimeout`, 300 s: past either, it and every
  process it started are stopped (`test/child.ts`'s `run`, its
  `memoryCap`), and it's kept as no failure, as a variant native Rust
  doesn't run is. Test: "a variant's check past its memory is stopped, and
  is no failure to keep" (`test/reduce.test.ts`), a variant holding 400 MB
  natively, refused within 2 GB and stopped within 200 MB.
- The nightly runs each test file of cases in parts by `scripts/parts.ts`
  alone, where a shell loop of `bun test .. &` printed nothing till every
  part ended: each part's lines as they come, its peak memory, it and what
  it started each second, and what held it then, said as it ends; the
  machine's memory each minute; and a part that a signal ends, or that's
  running as the job is stopped, said `killed (memory?)` with what it was
  running, each command's fixture naming its case. `bun test` holds a
  file's lines till it ends where its output is a pipe, so the last case a
  killed part said may be none: what it was running is where it was. Tests
  in `test/ci.test.ts`: a workflow runs a test file in parts by
  `scripts/parts.ts` alone, and a part a signal kills is said so.

The parts stay four at once: with a reduction held to its quarter, what a
part holds is its own runs, a native one at most 4 GiB on Linux. The
measured peaks of the next nightly are in TESTS.md.

Measured, nightly 42927113223 (2026-11-11): no part came near the
memory. The object model's four parts peaked at 0.5, 0.3, 0.3 and 0.3
GB in 8–9 min each; the memory net's at 0.3 GB each in 2–3 min; the
neighbour shuffle's at 0.4, 2.8, 0.3 and 0.6 GB in 10–34 min; and the
metamorphic net's at 0.4, 0.5, 2.3 and 0.5 GB, the 2.3 a reduction's.
The metamorphic net's parts, though, ran 141–149 min, and the job's
150-minute limit stopped them, so it runs on three machines of four
parts each, `1-4/12`, `5-8/12` and `9-12/12`, as the iterator matrix
runs on six (#1306).
