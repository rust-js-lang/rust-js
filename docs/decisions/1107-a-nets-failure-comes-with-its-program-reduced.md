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
