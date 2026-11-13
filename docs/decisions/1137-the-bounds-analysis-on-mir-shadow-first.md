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

## The triage, and the shadow's other side (2026-11-13)

- **The shadow now records both directions.** A line is written where
  THIR's facts leave out a check MIR doesn't prove: a wrong answer's
  candidate, or a gap in MIR's precision. A `mir-only-bounds` or
  `mir-only-wrap` line is written where MIR proves a check THIR keeps.
  These are the checks M3 starts leaving out, so each must be right
  before M3 does. Each hook, `shadow_check`, is told what THIR decided
  both ways.
- **A hazard found on that side:** `product_sign`'s `area(-2, 0)`. MIR
  proved `width * height` in range, but JS's own `*` is `-0` there, which
  no integer is (ADR 0064), and THIR keeps the product's normalization.
  A product proved exact now excludes one that may be `-0`: one factor may
  be 0 while the other may be below it, THIR's `may_make_negative_zero`.
- **Precision added:**
  - quotients by a divisor of one sign;
  - remainders by a positive divisor;
  - `&` of two never-negative values;
  - `x != c` narrowing an end equal to `c`;
  - a comparison narrowing every copy of what it compares
    (`let mut i = start`).
- **Triage of the 143 wrap and 18 bounds lines left:** no wrong answer
  was found. They fall into a few shapes, each a precision gap:
  - loop counters bounded through `Range::next` or a test of a field;
  - closures' captures and parameters;
  - values from a call's result;
  - indices checked against another slice of equal length;
  - products of two parameters bounded only together.

  Adversarial probes, each a variant with values that overflow, confirmed
  that THIR keeps the check:
  - a closure reached only through `Rc<dyn Fn>`;
  - recursive functions.
- **The mir-only lines** (314 bounds, 57 wraps), sampled: constant
  indexes; a remainder of the array's length; lengths of an allocation's
  items; `repr(u8)` discriminants; counters under a bounds check's
  `assert`. All sound, the `-0` product excepted, now fixed.

Measured: 143 wraps and 18 bounds, where M2b-2 left 162 and 18; 314
mir-only bounds, 57 mir-only wraps. Mutations `intervals-quotients-unknown`,
`intervals-remainder-reaches-its-divisor`, `intervals-unequal-unnarrowed`,
`intervals-negative-zero-exact`, `intervals-mir-only-unreported` and
`intervals-copies-unnarrowed`, caught.

## M3, step 1: bounds asserts, measured and held (2026-11-13)

MIR's intervals were switched to decide each bounds `assert`, by the
block it ends, of the very body proved. The switch was measured and then
held: THIR still decides, and MIR's proof is compared in the shadow.

- **Why held.** The lowering tests pin elisions THIR makes that MIR
  doesn't yet:
  - `xs[Math.min(i, xs.length - 1)]` and `xs[Math.max(i, j)]`;
  - `iter().enumerate()`'s index;
  - `heap[(index - 1) >>> 1]`;
  - `items[i + 1]` under `i + 1 < len`;
  - a closure's capture below two lengths;
  - `counts[i]--` of an element known positive;
  - rest arguments' `value[0]` under `value.length === 0 ?`;
  - `a[s - 1]` under `s >= 1` of a counter below a length.

  The switch lands once the shadow shows none of these, and no `bounds`
  line on the corpus, recovered through MIR's precision alone. In the
  measured run, the corpus passed against native with 23 checks left out
  that THIR kept and 16 kept that it left out.
- **A wrong answer the switch would have shipped.** `t = i - c` was taken
  below what `i` is below, though `i` may be under `c`: `p[i - 1]` under
  `i < p.len()` alone, at `i = 0`, read `p[4294967295]`, `undefined`,
  where native Rust panics. The relation now needs `i` known at least `c`.
  THIR keeps that check, so this was never on main. Case
  `index_one_below_a_value_that_may_be_zero`, and a shadow test.
- **Faults the shadow had hidden.** Its script ignored a crashed compile,
  and now fails on one:
  - the dominators of a transformed body's unreachable block panicked;
  - a `u128`'s range overflowed `i128`, and `-i128::MIN` overflowed. A
    128-bit integer is now never known.
- **Cost, for when MIR decides.**
  - A body with no bounds check and no `+`, `-` or `*` isn't walked.
  - The worklist visits blocks in reverse postorder, each queued once.
  - A loop no head dominates, of a transformed body's irreducible flow,
    widens anywhere after 12 visits, so the walk ends.

  Serde's `inbox`, decided by MIR, hung past 120 s, and with these takes
  15 s. Until the switch, a session with no shadow walks nothing.
- **Precision:**
  - **A `for` loop over a range of integers.** Its desugaring's
    `into_iter()` and `next()` are recognized by lang item
    (`recognition::range_step`). Each value is at least the range's start
    and below its end, of a range only `next()` changes: each `&mut` of it,
    or a reborrow, is `next()`'s argument alone, and nothing is written
    into its fields (`intervals/ranges.rs`). Case
    `range_end_written_before_its_loop` has one written a field at a time.
  - **`t = i + 1` of `i` strictly below a length:** at most that length.
  - **`len - c` of a length known at least `c`:** strictly below it.

Mutations `intervals-minus-one-may-wrap`, `intervals-range-next-unknown`,
`intervals-range-written-kept`, `ranges-field-writes-unseen` and
`ranges-reborrows-unfollowed`, caught.

## M3, step 1 landed: bounds asserts decided by MIR (2026-11-14)

MIR's intervals decide each slice's and array's bounds `assert`
(`bounds_proved`), by the block it ends, of the very body proved. THIR's
`in_bounds` is the shadow's alone for these asserts. Indexes made through
calls, a `Vec`'s `v[i]` and `swap`, are THIR's still, and are a later
step. Six of the eight gaps that held the switch are recovered through
MIR's precision. Two are lost for now, approved as readability costs and
pinned in their tests:
- #1700: `Vec::push`'s effect on a length;
- #1701: the parent's relations in a closure's captures.

What the walk learned:
- **Relations between values:**
  - `min` and `max` are below what either, or both, operands are below
    (`recognition::integer_min_max`, by diagnostic items);
  - `x / c` and `x >> c` of what's never negative are below what `x` is
    below;
  - `x + c` that never wraps is recorded, so a sum computed again is the
    first one, and `x + c < y` gives `x < y`: `items[i + 1]` under
    `i + 1 < len`;
  - `s <= len` and `s != len` give `s < len`;
  - a cast that keeps its value is a copy;
  - a comparison's relation reaches every copy of what it compares;
  - a narrowed length raises its slice's least;
  - `i - c` keeps `i`'s relations wherever it's computed exactly.
- **Intervals from relations:** a value below a length is at most what
  an allocation holds less one, so `i + 1` of `i < len` never wraps.
- **Joins:** a join keeps a relation one side has where the other side's
  intervals show it (`i = 0` before a loop), weakening `<` to `<=` where
  only one side is strict. A way no value takes, `i > 0` of `i = 0`, is
  no longer taken with what was known before it.
- **Loops:**
  - `rev()` and `step_by(n)` of a range;
  - `a..=b` (`RangeInclusive::new`, a lang item);
  - a slice's `iter()` or `iter_mut()` with `enumerate()`, and `rev()` of
    that: each index below the slice's length, of an iterator only
    `next()` steps (`recognition/loop_ranges.rs`).
- **Callers:** a relation between parameters at every call of a function
  only calls reach, `item(items, i)` under `i < items.len()`
  (`Given::below`).
- **Slices:**
  - a `&mut [T]` reborrowed, `v.swap(0, i)`, isn't taken as lent, since
    no write through it changes its length;
  - `&raw const (fake) (*v)` of a bounds check is the slice `v`;
  - a rest value's `Deref` is the same array.

The concept splits, each in a commit of its own: `intervals/state.rs`;
and, as new files, `recognition/loop_ranges.rs` and `intervals/ranges.rs`.

A range's record says what bounds its values as one `End`: below `b`,
at most `b`, a slice's items, or a slice's indexes. Two `bool` flags
could have said it in states no range is.

Measured: the corpus, all 2481 cases passing against native, has 31
outputs changed: 53 checks left out that THIR kept, and 19 kept that it
left out, each a precision gap with no wrong answer. Snapshots: 3 checks
left out, in collections, numbers and report. The guide's output is
unchanged, and the readability gate holds. Twenty-one mutations, caught.
