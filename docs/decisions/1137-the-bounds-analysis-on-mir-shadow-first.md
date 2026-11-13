# 1137. The bounds analysis on MIR, run in shadow first

Status: Accepted (2026-11-13, #1672, change 1 of architecture review #1671,
the user's decision to take it whole, #1681). M2a of four milestones: the
analysis and its shadow. M2b adds a small relational part, M3 moves each
consumer onto it, and M4 deletes `body_queries/ranges`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Nothing a program
observes changes: a flag a build never passes, and tests. A build's JS is
the same byte for byte.

## Context

The checks rust-js leaves out are left out on the word of
`body_queries/ranges`, an abstract interpreter of about 3,400 lines over
THIR. It models Rust's order of evaluation a second time, and MIR's
lowering trusts it through span joins. Two of its divergences from rustc's
order were wrong answers (#1659, #1660), and its joins were another
(#1683). ADR 0364 reads each order from rustc's MIR for exactly this
reason. The facts that license deleting a check should be read there too.

## Decision

**`analysis/intervals.rs` computes integer intervals of a body's locals by a
forward dataflow over the MIR the emitter reads.**
- Each integer local holds an interval, its type's range where nothing is
  known. wasm32's `usize` is 32 bits.
- A `bool` that a comparison made holds the comparison. It narrows the
  operands on each side of a switch of it, and past an `assert` of it. The
  comparison is of what a local copies: MIR compares `_7 = copy _3`, not
  `_3`.
- A local lent `&mut` or as a raw pointer is never known.
- At a loop's head, a bound still moving after three visits widens to its
  type's.

What it proves: each bounds check whose index is below its length's
least, and each `+`, `-` and `*` that stays in its type's range.

**It runs in shadow.** `--intervals-shadow <file>` logs each check that
THIR's ranges leave out and MIR's intervals don't prove: a bounds check by
its `assert`'s block, a wrap by its operator's span. `bun
scripts/intervals-shadow.ts` runs it over the corpus. Each line is a
wrong-answer candidate or a gap in this analysis's precision, and each is
triaged on #1672. What's left out doesn't change until M3 switches a
consumer.

A worklist over the body's blocks, not rustc's dataflow framework: a graph
and a table (#1671), which a reader follows without a framework's traits.

## Measured (2026-11-13)

Of the corpus's 2,453 cases, 401 checks that THIR leaves out aren't proved
by MIR's intervals alone: 63 bounds checks and 338 wraps. The ones read
were the relational kind, `i < v.len()` or `!s.is_empty()`, which an
interval of each side alone can't prove. That part is M2b's, kept small,
`x < len(p)` and `x <= len(p)`, as the plan says.

## Consequences

- `test/intervals-shadow.test.ts` holds the analysis: a loop's counter
  below its bound proved in range, a slice's relational index logged.
  Mutations `intervals-exact-unproved` and `intervals-copies-unfollowed`
  (`analysis/intervals.ts`), caught.
- No bounds check THIR leaves out is one intervals alone prove. THIR leaves
  out an array's index tested against its literal length nowhere in the
  corpus, and every slice's needs `i < len(s)`. So a mutation of the bounds
  proof waits for M2b and its test.

## M2b-1 (2026-11-13): the small relational part

Kept small, as the plan says.
- **What's tracked.** Each local that holds a slice's length, by the slice,
  through copies and reborrows (`&v`, `&(*s)`): from `PtrMetadata` and from
  std's `len()`, as recognition's `slice_length` takes it. Each `x < y` and `x <= y`, where `y` is a slice's length
  or an integer local. A least length per slice, from std's `is_empty()`
  switched on, a constant compared with the length (`b.len() > 4`), and a
  switch of the length (`match list.len() { 1 => .. }`).
- **What a relation proves.** A bounds check is proved by `index < len(s)`
  or by an index below the least. A subtraction `a - b` with `b <= a`
  known, or `b < a`, is at least zero or one. `t = i - c` keeps `i`'s
  relations, strictly where `c >= 1`.
- **Two fixes of M2a's walk, each found by its test.** A bound widens only
  at a loop's head, a back edge's target, so what a loop's test narrows
  stays narrowed past it. And a `falseEdge`'s or `falseUnwind`'s real
  target alone is followed, never the imaginary one, which joined an arm
  of a `match` with another's state.

Measured: 305 lines across 2,453 cases, 18 bounds and 287 wraps, where
M2a left 401. The bounds lines left are each a precision gap, as read:
`for i in 0..n` through `Range::next`, a closure's captured index, and two
slices of one length. Most wrap lines are of parameters, which THIR bounds
by what each call site gives a function only calls reach. That is M2b-2's,
over a call graph built from MIR (change 4 M3's).

`test/intervals-shadow.test.ts` holds each source. Mutations
`intervals-bounds-unproved`, `intervals-relation-unrecorded`,
`intervals-empty-unknown`, `intervals-widen-everywhere` and
`intervals-imaginary-edges`, caught.

## M2b-2 (2026-11-13): caller-given parameters, over one call graph from MIR

- **`call_graph.rs` is the crate's one call graph,** of the MIR the emitter
  reads, never the optimized one, where an inlined call is no call though
  the JS makes it. Each body's calls are resolved where rustc can; one
  through a `dyn` or a function pointer is `Unknown`. It carries its
  strongly connected components, callees first. Change 4 M3 moves the
  crate's other call-edge scans onto it.
- **Caller-given parameters.** A function only calls reach (`only_called`,
  ADR 0577) is given, as each parameter's interval, the join of what every
  call of it passes, as each caller's walk knows it. Callers are walked
  before callees. A function is walked knowing nothing of its parameters
  while some of its calls aren't walked yet, its own of itself among them,
  or are calls this MIR lacks: fewer calls seen than `only_called`
  counts. So one that calls itself knows nothing of its parameters as it's
  walked, and what it passes itself joins what it's given.
- **A length's bound.** A slice's or text's length, from `PtrMetadata` or a
  `len()` call, is at most `isize::MAX` bytes over its item's size, as every
  allocation is. So two lengths of `u8`s summed never wrap.

Measured: 180 lines across 2,453 cases, 18 bounds and 162 wraps, where
M2b-1 left 305. Mutations `intervals-given-unused`,
`intervals-given-before-every-call` and `intervals-lengths-unbounded`,
caught.
