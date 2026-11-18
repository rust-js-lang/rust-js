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

## M3, step 2 begun: wraps measured, two precision rules landed (2026-11-14)

Switching the wrap decision to MIR (`exact_proved`, by the statement's
location) costs ten elisions the lowering tests pin, so that switch is
held as bounds' was. Two rules it needs land first:

- **Narrowing.** After the widened fixed point, two passes in reverse
  postorder recompute each block's entry as the join of what flows into
  it, without widening. What a loop's test bounds, which widening at its
  head let go, comes back: `a[i]` past `while i < 5 { i += 1 }`. Each
  pass starts from a fixed point's flow, so it stays above what runs.
- **Lengths at calls:**
  - a JS string's `length`, a binding's, or `encode_utf16().count()` is at
    most `2^31 - 1` (ADRs 0632, 0682), as THIR's ranges have it;
  - text's `len()` (`recognition::text_length`) is at most what an
    allocation holds.

Bounds, which MIR decides, gain one check left out in the corpus
(`loop_ends_in_its_break`'s `times[j]`), and the snapshots are unchanged.
Mutations `intervals-unnarrowed` and `intervals-text-length-unbounded`,
caught.

## Four mutations that survived, resolved (2026-11-14)

At main, before the wrap switch, four mutations survived:
- **`bounds-join-any`.** Since f7f5bedf4, THIR's span join for bounds
  feeds the shadow alone, so its corpus case couldn't see it. It's now
  tested by a shadow test of the macro's repeated index:
  `["mir-only-bounds pair"]` exactly, where joining to any index would
  have THIR leave out `v[k + 1]`. Caught.
- **`intervals-copies-unnarrowed`.** Its test's copy no longer lived
  across the loop's head. A test now narrows a copy where it's read:
  `a[j]` of `let j = i` under `i < 10`. Caught.
- **`intervals-widen-everywhere`.** With narrowing after the fixed point,
  widening at every block gives what widening at loops' heads alone gave.
  Its mutant changed no output in the corpus (2,487 cases), the lowering
  suite or the shadow tests. So the walk now widens at any block after
  three visits, and the loop heads' dominator computation and the
  12-visit fallback for irreducible flow are gone. The mutation goes with
  the code it guarded.
- **`intervals-infeasible-edges-taken`.** Narrowing recomputes each
  entry from what flows in, so an infeasible edge's stale state never
  outlives the first pass. Its mutant was likewise equivalent across the
  same suites. The rule stays, being the semantics a walk should have (a
  way no value takes isn't taken), and the mutation is removed as
  equivalent (ADR 0093).

## M3, step 2 landed: wraps decided by MIR (2026-11-14)

MIR's intervals decide whether a `+`, `-` or `*` lowered from a MIR
statement is JS's own operator (`exact_proved`, by the statement's
location in the very body proved). Its wrap is left out where MIR proves
the value stays in its type's range, and isn't `-0` of a product.
Operations MIR's walk doesn't see keep what THIR's ranges know until it
does: a call's, `+=` of a trait or an atomic, generic number code, a std
body's overflowing forms.

Recovered through MIR's precision:
- **Narrowing past widening** (before this step), and widening at any block
  after three visits (the loop heads' rule went, being equivalent).
- **A sum computed again is the first one,** the same value though it
  wraps: `i += 1` past `i + 1 < n` takes that test's bound. And
  `x + c < y ⇒ x < y` holds of a sum that never wraps.
- **`x < y + 1 ⇒ x ≤ y`,** though `y + 1` wraps, its least being below
  nothing; of a comparison, and of a range's end, `k` of
  `for k in 1..line + 1`.
- **`t = x + c`** of `x + (c - 1)` strictly below a value is at most that
  value: `i += 2` past `i + 1 < len`.
- **A slice's `get(index)`:** its `Some` puts `index` below the slice's
  length, through `Option::cloned()` and `copied()` too.
- **A JS string's `length`** is at most `2^31 - 1`, and is a length of the
  text: a UTF-16 count is at most the bytes, so what's below it is below
  them. Text's `len()` is at most what an allocation holds.
- **Two locals compared record the relation their intervals show:** a
  loop's invariant, `n < limit`, that its first turn seeds.
- **An integer local lent `&mut` is forgotten where it's lent,** not in the
  whole body. Borrowck holds that it isn't assigned while the borrow lives,
  so once assigned again it's known. A raw pointer's local stays unknown
  in the whole body. While building this, the forget first came after an
  early return for a lent destination, which proved `k - 2` after
  `f(&mut k)`. The test caught it before anything landed.

Measured against main:
- **Corpus:** all 2,492 cases pass against native. 83 outputs changed: 110 wraps left out that THIR kept, 52 kept that it left out, each a precision gap.
- **Snapshots:** 6 files, a wrap left out in each.
- **Guide:** one wrap left out in `objects.md`; two kept in `advanced.md`, both in closures (#1701).
- **Lost, pinned in their tests:**
  - closures' captures (#1701): `catch_unwind` arms, `with_borrow`'s
    `get(i)`;
  - `Vec::push`'s length (#1700): `prevLength - 1`.
- **Gains the lowering tests had pinned as THIR's conservatism,** updated:
  - `skips`'s `i + 1 < items.len()`, `i` being at most the slice's length
    round the loop;
  - `index + 1` past `slots[index]`'s check;
  - a counted `for` over `text.len()`;
  - `usize::BITS - 1`, now JS's own `32 - 1`.

A literal fold tried in `binary` broke the tidy pass's merge of
`Some(x) | Some(x)`'s guards, adding renames to `match_trees_chained`, and
was dropped. The readability gate holds.

The concept splits, each a commit of its own:
- `intervals/calls.rs`, what a call tells the walk;
- `test/lowering-integer-ranges.test.ts`, out of `lowering-numbers`.

## M3, step 2 amended: widening to thresholds, a push's length, and the bridge (2026-11-14)

The wrap switch regressed 9 lines of React's port, wraps THIR's ranges left
out that MIR didn't prove. They came from five gaps.

Two are recovered by MIR's precision:
- **Widening to thresholds.** A bound still moving after three visits
  widens to the next constant the body compares an integer with (each one
  less and one more too), and only past them all to its type's. A counter
  a loop breaks out of, `if count > 10 { break }`, that a `continue` brings
  back to the head unchanged, widened to its type's, is a fixed point no
  narrowing leaves: the `continue` edge carries the widened bound back.
  Widened to 10, the test holds it there. This recovers `estimateBandwidth`'s
  `count++` and `addObjectToProperties`' `addedProperties++`. The
  thresholds are finite, so widening still ends.
- **A push's length** (#1700). Past `push`, a `Vec`, held by a local or
  behind a `&mut` one, holds an item at least, until it's lent `&mut`
  again or written. A shared reborrow of it, as `len()` takes, carries the
  fact. A local a raw pointer is made of anywhere in the body is never
  known pushed, as the pointer may write it after. This recovers
  `properties[prevLength - 1]`.

The other three are more than a day of MIR precision each:
- `Ref`s of one `RefCell` sharing a length (#1741);
- a closure's result telling its caller nothing, and a `&mut` capture's
  place (#1701).

For those, **the bridge**: where MIR doesn't prove a wrap that THIR's ranges
know never wraps, THIR's verdict. Both are sound, so their union is too.
- **Where:** exactly one place, `binary_proved`, the consumer of a wrap
  verdict.
- **The list:** each site the bridge decides is a `bridged` line of the
  shadow. test/intervals-bridged.txt lists each one in the corpus, with
  the issue that would prove it (#1701, #1741, #1742). Its test fails when
  a site isn't listed, or a listed one no longer is. A new one is a gap to
  recover first.
- **The exit:** M4 deletes THIR's range code and the bridge together, once
  the list is empty.

Corpus case bridged_port_shapes holds the port's three shapes the corpus
lacked: a `Vec` behind a `RefCell` indexed at `len() - 1` past
`!is_empty()`; a `&mut`-captured counter in `catch_unwind`; and
`with_borrow`'s `get(i).cloned()`.

## M3, step 3: casts decided by MIR (2026-11-14)

An integer `as` another is the number it casts, written as it is, where
MIR's intervals prove its value in the target's range (`Proofs::casts`,
`cast_proved`, by the statement's location). THIR's range-proven casts are
now the shadow's alone (`range_casts`, a `cast` or `mir-only-cast` line);
THIR's other kept casts stay, as they aren't range code: a float known whole
(`whole_casts`), and the signed count of text's `repeat`.

- **Both of 32 bits at most.** A 64-bit integer is a `BigInt`: its value
  may fit, `n as u64` of `n < 200`, but the JS value isn't the same. The
  first cut proved those too, and 12 corpus cases threw "Cannot mix BigInt
  and other types" against native, as did the regenerated std. The corpus
  caught it before anything landed.
- **A body whose only question is a cast is walked.** The walk had been left
  out of a body with no bounds check and no `+`, `-` or `*`, so a `size_hint`'s
  `self.0 as usize` was proved nowhere; with it, 127 of the first 128
  losses went.

Measured against main:
- **Corpus:** all cases pass against native; 6 outputs changed.
- **Gains:** a length as an `i32`, below what an allocation holds
  (`rest.length`, `$byteLen(kind)`, no `| 0`); and `i * 7 % 251` as a `u8`.
- **Lost:** one, `lanes as u32` in a closure (#1701), now
  `clz32(lanes >>> 0)`, the same value.
- **Snapshots, guide and std:** unchanged.
- **The port:** it couldn't be regenerated. On main, `js::validate`
  (eeeffeafa) refuses two of its files, for reasons unrelated to casts.

## M3, step 4: a `OnceCell` still empty, decided by MIR (2026-11-14)

A `OnceCell`'s `set` is an assignment where MIR shows its cell still empty
there (ADR 0847), each time it's reached (`analysis/once_empty.rs`,
`empty_sets`). It's a dataflow of its own, beside the intervals, run only
on bodies that call `set`. Its rules are THIR's:
- **A cell** is the local whose value holds it and the fields to it,
  through `Rc`s and `Box`es: `(fresh, [1])` of `fresh.cache`.
- **What a move, a struct or a tuple, or `Rc::new`/`Box::new` gives a
  local** holds what its operand held there.
- **A reference** is the place it lends; an `Rc`'s `Deref` lends what the
  `Rc` holds.
- **An `Rc`'s clone** reaches the cells of what it's cloned from.
- **What reaches a cell** is used any other way than these: given to a
  call, captured, lent raw, or dropped where dropping it may run code
  (`drop_runs_code`, moved here). Then every cell it reaches is forgotten,
  as it is where the cell is set.
- **A join** keeps what both ways know empty.

THIR's `set_empty` is now the shadow's alone (an `empty-set` line). In shadow
first, the corpus showed no disagreement: MIR proves each of the 11 sites
THIR did, in once_cell_known_empty.rs and once_cell_fields.rs, and no
others. The corpus, snapshots and port are unchanged. THIR's 11 once-cell
mutations went, each with a MIR twin; 11 new ones of MIR's are caught.

## M4: the bridge's gaps closed (2026-11-14)

Each gap closed takes its sites off test/intervals-bridged.txt in the same
commit:
- **An outer loop's index across an inner loop** (#1742). The widening's
  thresholds take each collection's most length too, and one less. An
  index that grows by the outer loop's turns, widened at the inner loop's
  head, stops at the most `get`'s `Some` puts it below, not its type's,
  which no narrowing would leave. 1 site.
- **A generic constant** (#1742). A constant that doesn't evaluate, `N` of
  `while i < N`, is a value of its type's range (`Value::Range`): `i < N`
  puts `i` below the type's most, and `i >= N` puts it at least the type's
  least. 1 site.
- **A function that calls itself** (#1742). A component of the call graph
  that calls itself is walked to a fixed point of what its own calls give,
  from what its callers outside give. A member nothing has given anything
  yet isn't walked that round, so two that call each other aren't seeded
  with nothing known. Past four rounds, what its own calls still change is
  forgotten (ADR 0577's rule); past 64, all of it. 1 site.
- **Constants' callers** (#1742). Each constant's MIR, a `const`'s, a
  `static`'s or an inline `const { .. }`'s, the MIR rustc evaluates it by
  (`mir_for_ctfe`), is a caller in the call graph. A `const fn` only
  constants and patterns call, `add(1, 2)` of `const SUM`, is given what they
  pass; one a constant calls with large values and a body with small ones
  is given both. 2 sites, #1742's last.
- **Refs of one `RefCell`** (#1741, `intervals/cells.rs`). A `Ref` a cell's
  `borrow()` gives, and what the `Ref`'s `Deref` lends, are copies of the
  cell, so a length read through any borrow of it is the cell's:
  `stack.borrow()[stack.borrow().len() - 1]` past `!is_empty()`. Code that
  can reach a shared `&RefCell` may write it, so what's known of a cell's
  length is forgotten at any call but a `borrow()`, a `Ref`'s `Deref`, a
  length, an `is_empty()` or an index's read; at a drop that may run code;
  and at a write through a raw pointer, which reaches a cell's value with no
  call. THIR's ranges saw no such write: the bridge left such a site's wrap
  out, unsound, until #1782 kept the bridge out of such bodies. 1 site.

## Amendment: a local lent `&mut`, then tested (2026-11-14, #1782)

Step 2's rule forgot an integer local lent `&mut` where it's lent, not in
the whole body, and learned it again by what came after. A test after the
borrow taught it something, but the `&mut` may have become a raw pointer,
or been given to a call that keeps one, that writes it later. A read through
the owner leaves such a pointer able to write. `n - 1` past `n != 0` gave
`-1` where the pointer had set `n` to 0, and Miri finds no undefined
behaviour there. THIR's ranges took the same wrong step, so the bridge did
too.

Now:
- **No test tells.** A local forgotten at its borrow (`lent_away`) is
  narrowed by no test, and related by no comparison, until it's assigned
  again.
- **The bridge distrusts such bodies.** It doesn't take THIR's verdict in a
  body that makes a raw pointer or lends an integer local `&mut`
  (`thir_trusted`).

Corpus case raw_pointer_written_after_its_owner_reads holds both. The
corpus, snapshots and port are otherwise unchanged. Found with it: #1781,
where a pointer a callee keeps of a `&mut` writes the caller's box, which
the caller had copied back out, so the write is never seen.

## Amendment: a closure's captures (2026-11-14, #1701 A)

A closure's body is its own MIR body. It reads what it captures as fields
of its first parameter, `(*_1).k`, and of a reference it captures, the
place it refers to, `*(*_1).k`. The walk knew nothing of either, so 17 of
the bridge's 24 sites were wraps of a capture: `move || at * 2` of
`at = 10`.

Now a closure's captures are given where it's made (`capture_intervals`):
- **By copy, its value there.** It doesn't change unless the body writes
  it (`captures_written`): `at -= 1` of a `move` closure's own `at` keeps
  its wrap.
- **By shared reference, what it refers to there.** Borrowck holds that
  unchanged while the closure lives. A `&mut` capture is given nothing.
- **Joined over each place it's made.** Its makers are walked first, by
  how deep in closures each is; one made where a closure as deep or deeper
  is, an inlined copy, is given nothing.

17 sites left the bridge's list; 7 remain, B's and C's.

## Amendment: a closure's captures as locals (2026-11-14, #1701 B)

A's captures are known where the closure is made, but a test in the body
couldn't narrow one. `from_fn(|| ..)`'s `n -= 1` past `n == 0`, of a
`&mut n` capture, reads and writes `*(*_1).0`, which is no local, so the
walk learned nothing from the test.

Now the walk reads a closure's body with each capture a local of its own
(`as_locals`). The blocks and statements are the body's, each where it
was:
- **What a reference it captures refers to,** `*(*_1).k`, where the body
  only ever reads it through the reference.
- **The field, `(*_1).k`,** of any other capture.
- **Not at all** where the body reads itself whole, which may reach any
  capture.

The walk's rules of a local are then the capture's, one rule and not a
second copy for closures. A test narrows it, and a copy is one of it. A
`&mut` of it lends it: `bump(&mut n)` forgets `n` as #1782 forgets a lent
local, so `n -= 1` after keeps its wrap. A raw pointer or a `Box` it
captures stays a field, as other code may reach what it points at. A's
givens become each local's interval where the body starts: a shared
reference's referent, and a copy the body never writes.

5 more sites left the bridge's list; 2 remain, C's.

## Amendment: a closure's result told its caller (2026-11-14, #1701 C)

The last 2 sites were `while let Some(x) = KEY.with_borrow(|v| v.get(i).cloned())`
then `i += 1`. The `Some` knows `i` is below a length inside the closure,
but the caller, past the loop's test, knew nothing of it.

Now a closure's result is summarised (`closure_results`): one whose `Some`,
at every return, is a slice's `get(index)` of a capture gives the capture's
field and the most an index below that slice's length is. A thread-local's
`with`, `with_borrow` and `with_borrow_mut` (`local_key_with`) give what
their closure gives. So the caller's result is a `get` of the local the
capture refers to, which its `Some` narrows as a direct `v.get(i)`'s does.
The local is the caller's own:
- **by a shared reference,** which borrowck holds unchanged while the
  closure lives;
- **or by copy,** of a closure made in the block whose call it's given,
  where nothing after it's made writes the local.

The bridge's list is empty, so M4 can delete THIR's range code, the bridge
and the list. One mutation is equivalent and not kept: a closure's MIR has
one return block, so the join there already drops a `Some` of anything
else, `if i < 2 { Some(0) } else { v.get(i).copied() }`.

## M3, step 5 begun: a call's indexes, in shadow (2026-11-14)

THIR's facts still decide one thing outside the bridge: the indexes made
through calls. That covers a `Vec`'s `v[i]` and `index_mut`, `swap`,
`swap_remove`, `remove` and `insert`, `&v[a..b]`, and a map's `m[k]`.
M4 can't delete THIR's range code until MIR decides them. In the corpus,
THIR's facts decide 219 such calls.

The first step proves them in shadow, and nothing is decided by it yet:
- **The proof.** A call that checks its indexes against its receiver's
  length (`call_indexes`) is proved where each index is below the length,
  or at most it for `insert` (`Proofs::call_bounds`, by block). Each site
  is compared with THIR's facts as a `call-bounds` line.
- **A `Vec` lent only to calls** (`called_on`). A local holding a `Vec`, or
  a `&mut Vec` pointing to one, was lent for the whole body by any `&mut`
  of it. Now one whose `&mut`s are given only to calls isn't. A call that
  may change its length, `push` or `clear()`, forgets the length past it.
  One that keeps it, `index_mut`'s `v[i] = x` or `deref_mut`, keeps it. A
  `Vec`'s `deref` is a slice of its length, a copy of it for its length,
  so `v.swap(i, j)` is of the `Vec`'s. What a call does with a `&mut` it's
  given happens before the `Vec` is used again, as borrowck holds. A
  `&mut` moved to another local, or read any way but given to a call or
  reborrowed, lends the `Vec` as before.

The shadow's `call-bounds` lines fall from 212 to 202. What remains, in
the order the lead approved:
- **range indexes,** about 100 sites: `X[..]`, `X[1..]`, `X[..n]`;
- **field places as roots:** `self.items[self.i]`;
- **the map's key-held fact,** as a MIR dataflow of its own;
- **lengths related across two `Vec`s:** tables.

Main's survivors are caught or reasoned here:
- `intervals-relations-bound-nothing` is now held by "MIR decides a
  bounds check" (intervals-shadow), as thresholds alone hold its old test;
- `intervals-unnarrowed` is held by "an index below a generic constant";
- `inclusive-for-unranged`, THIR's, goes (ADR 0093 amended).

### Step 5: a range's bounds (2026-11-15)

`&v[..n]`, `&v[n..]`, `&v[a..b]`, `&v[..=b]` and `&v[..]`, a `Vec`'s, a
slice's or an array's `Index` by a range of `usize` (`range_index`), are
proved where:
- **the end** is at most the length, or below it of `..=b`;
- **the start** is at most the end, by their intervals or a relation, of
  `a..b`, or at most the length, of `a..`.
`&v[..]` is always in bounds. Text's ranges aren't proved, as their bounds
are on its characters too. A range's bounds made through std's text path,
`slice_range_values`, are compared in the shadow too.

The shadow, measured anew with the call-bounds lines where THIR's facts
decide a call's indexes or a range's: 146 THIR-only, and 103 MIR-only, each
sound:
- `&v[..]`, which THIR's facts never marked;
- `insert(0, x)` and `&v[..0]`, at most any length;
- a stepped loop's `v[i]` below `v.len()`;
- a `swap` of a slice whose earlier bounds checks show its length.

### Step 5: a struct's fields as locals (2026-11-15)

A method's `self.items[self.at]` read its fields through `&mut self`,
places of `(*_1).f`, which the walk knew nothing of. Now the walk reads a
body with each field a reference parameter to a struct is read through as
a local of its own (`fields_as_locals`), as #1701 B made a closure's
captures. The walk's rules of a local are then the field's:
- a test narrows it;
- a copy is one of it;
- a `&mut` of it lends it;
- a `Vec` field lent only to calls keeps its length: `self.0.remove(0)`
  past `!self.0.is_empty()`.
A parameter given on whole, `self.reset()` or `&mut *self` given a call,
may change any field, and its fields stay places.

The shadow's call-bounds lines fall from 146 to 132. Beyond them, MIR now
proves 173 more wraps than THIR's facts did, mir-only-wrap 103 to 276,
each a field stepped under a test, such as `self.end -= 1` under
`self.at < self.end`. Since MIR decides wraps (M3 step 2), the JS has them
as JS's own: 117 corpus cases, each passing against native, and
@rust-js/std's `iter.js`, where `take.n - 1` is under `take.n > 0`.

### Step 5: an index a loop takes from its slice's `enumerate()` (2026-11-15, #1814)

A downstream port's `candidates[best]` kept its check. It reads:
`let mut best = 0; for (index, c) in candidates.iter().enumerate().skip(1)
{ .. candidates[best] .. best = index }`. Two rules:
- **`skip(n)` is a step of a range** (`RangeStep::Skip`). Of a range of
  integers, a slice's iterator or its `enumerate()`, it gives the rest of
  its values, each below the same end. So `candidates[index]` under
  `enumerate().skip(1)` is proved.
- **A join keeps a guarded relation** (`State::below_past`). Where one way
  knows `x` below a slice's length and the other knows `x` at most `c`,
  `x` is below the length wherever the slice holds more than `c` items.
  That's so in the loop's body: there `index` below the length shows the
  slice holds one at least (`least_of`). `best = 1` before the loop isn't
  proved, as a slice of one item doesn't hold it.

The corpus's one change is enumerate_mut_patterns' `i + 1` of an
`enumerate().skip(1)` index, written without its cast.

### Step 5: a `RefCell`'s guard (2026-11-15, #1815)

A history a `RefCell` holds kept its checks: `queue.borrow_mut()[i] = x`
under `i < queue.borrow().len()`, and `items.remove(i)` past a callback
`f(items[i])` while `items`, a `borrow_mut()` guard, is held. #1741's rule
forgot each cell's length at any `borrow_mut()` and at any call. Now ADR
0881's rules are MIR's too (`guards`):
- **A held guard guards its cell** (`State::guarded`). A guard is held
  where only its own `deref` and `deref_mut` read it and its drop ends it,
  and a `RefCell`'s own `borrow_mut()` made it. A function of the
  crate's that returns a `RefMut` may give one of another cell, so its
  `RefMut` isn't held. While a guard lives, any other borrow of the cell
  panics, so a call, or a drop that runs code, can't change what the cell
  holds. The guard and its `deref` are copies of the cell, for its length.
  At the guard's drop the cell isn't guarded any more.
- **What the guard's `deref_mut` lends** keeps the length only past a call
  that keeps it, `index_mut` or a `deref_mut` to a slice. Given to
  `truncate`, `push` or a function of the crate's, the length is
  forgotten. Used any other way, a write through it, `*items =
  Vec::new()`, or one moved on, the length is forgotten at the
  `deref_mut`. A reborrow of it, `&mut (*m)`, given only to calls is it.
- **An index `v[i]` gives its item at is below the length** past the call,
  as the call panics otherwise. So a later `v[i]`, or a drain to it, is
  in bounds.
- **A `Ref`'s or a `RefMut`'s own drop runs no code**: it gives its
  borrow back and drops none of what the cell holds. Before, a guard of a
  `Vec<Rc<dyn Fn()>>` was taken to run its items' drops.
- **A `Vec`'s `drain(a..b)`** is a range proved as `&v[a..b]` is.

The shadow's call-bounds lines fall from 132 to 110. Of the cells'
cases, `refcell_listeners_momentary`'s one line remains: its cell is
reached through an `Rc` a closure captured, each `listeners.borrow()` a
`deref` of the `Rc` to a new `&RefCell` local, which isn't a copy of the
last. That's #1839. MIR now proves 20 call-bounds sites
THIR's facts keep, each sound:
- `v.drain(..)`, always in bounds;
- a guard's `items[i]` under its loop's range;
- `bounding_rects[j]` of `j < i < len`;
- `flush`'s `queue.borrow_mut()[at]`, as its callers give `task` at
  least one.
Two wraps it proves are new, `i += 1` past `signs[i] = -n`, now JS's own
in `index_written_by_an_operation`. `refcell_guard_bounds`, a new case,
runs each way the guard's length is forgotten against native Rust, each
panicking as Rust does.

### Step 5: a map's key held (2026-11-15)

THIR's facts decided a map's `m[k]` (rd558) with no MIR counterpart and
no shadow line: it was never compared. Now the shadow compares each map
index, and MIR proves it too (`keys`):
- **`contains_key(&k)` switched on** shows the map holds the key on its
  `true` way (`State::has_keys`, then `State::keys`), by the roots of the
  map and the key. A `RefCell`'s map is its cell's, as a `Vec`'s is.
- **An `m[&k]` that gave its value** holds the key past it, as it panics
  otherwise.
- **What forgets it:** a write of the map or the key, a `&mut` of the map
  lent, and of a cell's map, any call or drop that may write the cell
  (`forget_lengths`).
- **Of a key whose lookup reads the same until it's written** alone
  (`key_stays`, #1844, ADR 0059 amended). A key whose hand-written `Hash`
  reads a `Cell`, or a static, may change through a shared reference.
- **A body is walked** where its one check is a map's index.

All the corpus's map indexes THIR decides are proved too, but two. Both
are `Rc<RefCell<HashMap>>`s a closure captured,
`map_index_of_a_key_held`'s `handle_error` and
`closure_function_comments`'s. Each `borrow()` is of a new `&RefCell` an
`Rc`'s `deref` gave, the shape #1839 tracks. So the shadow's call-bounds
lines go from 110 to 112: two map sites now compared, not two lost. MIR
proves a second `m[&k]` of the same key where THIR keeps its check.

### Step 5: a table's lengths (2026-11-15)

THIR's facts decided 37 call indexes of tables MIR didn't prove (ADR
0576): `vec![x; n]`'s, a table of rows' and an array's. Now MIR proves
them (`tables`):
- **An array is as long as its type says**, `&keys[..6]` of a captured
  `[i32; 11]` (`index_below`'s least).
- **`vec![x; n]` is as long as `n`:** `n` holds its length, or where `n`
  is another slice's length, the two are as long (`State::equal_lens`):
  `vec![0.0; terraces.len()]` and `terraces`. An index is below a slice's
  length below any as long, or below an integer that holds it, `i < count`
  of `vec![0; count]`; and what holds its length gives its least, `board[0]`
  where the callers give `height` at least 2. Those caller facts are only
  of a function `only_called` reaches (ADR 0577), not one taken as a value.
- **A table of rows is known by its row** (`State::rows`),
  `vec![vec![1; width]; height]`: `board[r]` is as long as the row it was
  made of, so `board[r][c]` under `c < width` is proved. That holds only of
  a row type no `&mut` in the body may resize (`rows_kept`):
  - each `&mut` row is read only as an `index_mut`'s or a `deref_mut`'s
    receiver, a shared borrow, or a reborrow read so;
  - a closure or a function reaches a row only through such a `&mut`,
    or a call given the table's `&mut`;
  - a call given the table's `&mut`, or its slice's, forgets its rows, but
    one that keeps them, `index_mut`, `deref_mut`, an index or a length.
  So a row written whole, `board[0] = vec![]` or `*board.index_mut(0) =
  ..`, swapped out, taken, `fill`ed or `swap`ped, keeps its checks, as
  #1849's `table_rows_resized_unseen` runs against native.

The shadow's call-bounds lines fall from 112 to 31: the tables' 31, and
beyond them an array's typed length and an index below an integer that holds a
slice's length prove 50 more across 32 cases. These are `&bytes[..len]` of
an array, `v[i]` of `vec![0; n]` under `i < n`, and `&v[..n]`. What a table
lends for a view, `board.iter_mut()`, forgets its rows, so the 4 lines of
`table_rows_kept_by_views`' loop that only writes cells stay THIR's
(#1852). Eight sites are newly MIR's, each sound:
- of an array's type, `&bytes[..len]` past `bytes[len] = b`,
  `[5, 6, 7, 8][1..]`, `&PAST_THE_LIMIT[..16384]` of a `[u8; 16385]` and
  `[guard][0]`;
- `right_max_levels[i + 1]` under `i < terraces.len() - 1`;
- `pattern_table[prefix_index - 1]` past `p[prefix_index]`, as long as `p`.

### Step 5: triage before the switch (2026-11-15)

Of the 31 call-bounds lines left, three small rules close five, and a
sixth closes a readability report:
- **A test held in a `bool`**, `let truncated = len > LIMIT; if truncated`,
  is switched on as its copy. The switch looks the comparison up through
  the copy's root (`State::comparison_of`). A negated one, `!short`,
  carries the comparison inverted through the `Not`
  (`Comparison::negated`). What it compares written between forgets it, as
  any comparison of a written local is forgotten. So does a slice whose
  length changes between: the length's temporary no longer holds it
  (`slice_ranges_known`'s `cleared` and `rewritten`, each panicking against
  native).
- **A shared reference to a struct that holds no cell itself** keeps its
  fields as locals where it's also used whole, returned or compared
  (`fields_as_locals`): nothing changes its fields while it's borrowed.
  That proves `find(&node.children[0])` under `node.children.len() == 1`
  beside `node` returned. A `&mut` one used whole, `reset(node)`, doesn't.
- **A `bool` made an integer**, `usize::from(apple)` or `apple as usize`,
  is 0 or 1 (`integer_from_bool`). So of an array of two its index is JS's
  own, `parsed[apple ? 1 : 0]`, through a thread-local's `with` too
  (#1863). Of an array of one, `true` is past it, checked
  (`bool_indexed_arrays`).

### Step 5: an `Rc`'s cell (2026-11-15, #1839)

An `Rc`'s or an `Arc`'s `Deref`, std's (`rc_deref`), gives what it points
to, the same each time, and runs no code. So of a local the body neither
reassigns nor lends `&mut`, it is a copy of that local, as a `borrow()` is
(`intervals/cells.rs`), and it isn't a call that may write a cell. That
proves React's `listeners.borrow()[i]` under `i < listeners.borrow().len()`,
and `deletions.borrow()[i]` of a `&Rc`, as THIR proves them. A local
reassigned forgets what's known of it, and one lent `&mut` is no copy's
root, as before.

The cells whose lengths a call forgets are now the locals that point to a
`RefCell` through references, `Rc`s, `Arc`s and `Box`es, any depth of them
(`holds_cell`), where they were a `RefCell` and its references alone. A
`Box`'s cell was already a copy's root through `&(*b)`, so past a closure
that clears it, `items.borrow()[i]` was proved by MIR, a
mir-only-call-bounds line THIR's check kept from shipping; it's checked
now, as an `Rc` of an `Rc`'s is. A `Deref` of the crate's isn't an `Rc`'s,
and may give another cell each time. Each row panics as native does
(`rc_refcell_bounds`). Of the corpus, the call-bounds lines go from 26 to
23, and no line is added.

### Amendment: a write the walk doesn't see (2026-11-15, #1877, #1878)

The audit before the switch found two writes the walk didn't see, each a
live wrong answer, since MIR decides bounds asserts and wraps:
- **A write through a reference**, `*v = Vec::new()`. A statement whose
  place isn't a local returned before anything was forgotten, so
  `v.len() - 1` under `!v.is_empty()` stayed known not to wrap (#1877).
  Now a write to any place through a deref forgets what's known of the
  length its root reaches. That covers a deref of a deref, `**v = ...`, a
  write through a raw pointer, `ptr::write`'s too, and a field's place
  through a reference. An item's write, `v[i] = x` of a `&mut [T]`,
  can't change the length, and keeps it. A table's rows are reached by
  `Vec`'s `IndexMut` alone, never a place's index. A call's result reaches such a place through a
  temporary, moved in by an assignment, so no call's destination is a deref
  place in the MIR the walk reads. A `&mut` given to a call was already
  forgotten (`called_on`).
- **A parameter written**, `h = other` of a `&Pair` (#1878).
  `fields_as_locals` took a shared reference's fields as locals where it's
  used whole too, as nothing changes them while it's borrowed. But the
  reference itself can be made another's, and its fields are then that
  one's. A parameter the body writes, by an assignment or a `&mut` of
  it, has no field locals now.

`test/bounds-matrix.ts` is the net these came from. It takes each holder
of a collection: an owned `Vec`, a slice, a `&mut` of each, a `&mut &mut`,
a `RefCell` and its guard, an `Rc`, a `&Rc`, a `Box`, a thread-local, a
struct's field of each kind, a local struct's, a table's row and a raw
pointer's pointee. Each meets each test: below, held in a `bool`, the
next, the last, a wrap, a `while` and a `for` loop, in a closure, and in a
callee. Each test then meets each thing between that may shorten it: a
callback through another path, a drop, a closure, a direct call, a
reassignment, a `&mut` lent, the whole written, a raw pointer's write.
Each meets each index form, an item, a range's end and `swap`, and runs
as native Rust does. What Miri finds undefined behaviour is left out,
with Miri's words beside it in the matrix: a raw pointer's write that
invalidates a closure's capture, and a `swap`'s `&mut` reborrow that
invalidates the pointer a `for` loop writes through next. Under the switch's build, 12 of its first 104 programs
differed, and with the fixes none does.

### Amendment: a held guard's `deref_mut` is a copy (2026-11-15)

Since #1877 a write through a reference forgets the length it reaches, so a
held guard's `deref_mut` no longer forgets the cell's length itself where
its `&mut` isn't only given whole to calls. It's a copy of the cell, as the
guard's `deref` is. What changes the length through that `&mut` forgets it
where it does: a write through it, a call given it (`guard_lent`,
`called_on`), the guard lent `&mut` where it's borrowed. No row told the
old branch apart (ADR 0093, amended). The bounds matrix's `guard` holder
keeps the shapes it covered as rows: the `&mut` kept in a struct's field
and the `Vec` shortened through it, and a raw pointer made of it, shortened
by a call and replaced by `ptr::write`. The guard shadow test keeps the
struct field's.

### Amendment: a held guard's `deref_mut` forgets again (2026-11-15, #1928)

The amendment above, a held guard's `deref_mut` a copy, is undone. Its `&mut` bound to a local and shortened through it, `let all = &mut *items; all.truncate(1)`, reaches the call as a reborrow of the local, `&mut (*all)`. No root leads that back to the cell, and the guarded cell is skipped by the calls-may-write-cells rule, so only the `deref_mut`'s own rule forgets its length. Without it, `(items.len() - 2) as u32` under `len > 1` gave -1 on main. The bounds matrix's `guard` holder has the shape now (`bound`). The general rule should see through that reborrow, so that a call through it forgets the cell: #1929. Until then, the branch stays.

## Amendment: the bridge is gone (2026-11-15, #1943, M4's first part)

The bridge took THIR's ranges for a wrap MIR's intervals don't prove,
except in a body that makes a raw pointer or lends an integer local
`&mut` (`thir_untrusted`, #1782). A raw pointer the body is *given* was
unseen: under `!cell.borrow().is_empty()`, `*p = empty` through a
`*mut Vec<u32>` parameter then `(cell.borrow().len() - 1) as u32` was
JS's own `-`, `-1`, where Rust gives `4294967295`. MIR's cells rule
forgets the cell's length at the write and keeps the check; THIR's
ranges, which see no write through a pointer, proved it, and the bridge
took that. Its list, test/intervals-bridged.txt, was already empty: no
corpus case needs it. So the bridge goes, with `thir_trusted`,
`thir_untrusted`, the shadow's `bridged` line, the list and its test: a
wrap is JS's own where MIR proves it, and nowhere else. THIR's ranges stay
for what they still decide, a call's indexes, until step 5's switch; the
same hole there is #1944, which the switch closes. Held by the bounds
matrix's `emptied through a pointer` row, which differed from native
before, and catches cells-raw-write-quiet now that no other rule decides
its site.

## M3, step 5 landed: MIR decides a call's indexes (2026-11-15, #1944)

A call's indexes are MIR's now, as its bounds asserts, wraps and casts
were: a `Vec`'s `v[i]`, `remove(i)` and `insert(i, x)`, a range's
`&v[a..b]` and `drain(a..b)`, and a map's `m[k]` of a key held are
unchecked only where `call_bounds_proved` says MIR's intervals prove them
(`call_bounds` of the body's proofs), not where THIR's ranges did
(`indexes_in_bounds`), which the shadow alone still asks. THIR's ranges saw
no write through a raw pointer a body is given: `cell.borrow()[i]` under
`i < cell.borrow().len()`, past `*p = empty` that emptied the cell's
`Vec`, read `undefined` where Rust panics (#1944, the same hole as #1943's
wrap). MIR's cells rule forgets the cell's length at the write, and keeps
the check.

Before it landed, the audit's ablation against native (ADR 0093's rule),
on the switch rebased on main: the bounds matrix and the corpus, its
snapshots ignored, all as native runs them; the ten mutations of the
unsound direction, eight caught by `shape_unsound_directions`' rows and
two answered on #1672 (sound as written; a shape only Stacked Borrows'
undefined behaviour reaches). 36 corpus snapshots change, each run as
native does; React's port, 30 more checks than it had, each where THIR's
ranges proved what MIR doesn't yet (a `Vec::remove`'s guard, #1814's
shape; the Flight node loaders' `exportedEntries[nextEntryIdx]`;
`SchedulerMinHeap`'s; and five fewer, where MIR proves an index THIR
didn't), the pin moved with them. Mutations call-bounds-proved-of-every-call
(the matrix's `indexed through a pointer`, #1944's row) and
call-bounds-proved-of-none (a snapshot's) hold it. M4 deletes THIR's
ranges.

## Amendment: a closure's `&mut` lends only to its calls (2026-11-15, #1946)

`called_on`, a `Vec` lent `&mut` only to the calls it's given to, took a
`Vec` alone, and counted borrowck's `FakeRead` of a `let` as a use that
lends it: `let r = &mut v;` and `let mut take = || args.pop_front();`
each lent the local for the whole body, so `args[i]` under
`for i in 0..args.len()` past `take()`'s last call was `$index(args, i)`,
a readability promise lost at step 5's switch (Console's loops). Now a
`VecDeque` is one too; a closure that captures such a `&mut` holds it, and
`&mut take` given its `call_mut` lends it to that call alone, which
forgets the length; and a `FakeRead`, which neither writes nor copies,
lends nothing. borrowck holds the rest, as it does for a `Vec` given to a
call: nothing reads the `Vec` while the closure that borrows it is used
again. Held by the bounds matrix's `taken by a closure` row, a length read
before the closure pops and the index under it after, and by mutations
of each part.

## Amendment (2026-11-15, #1945): `position()`'s index is below its slice's length

Since MIR's intervals decide a call's indexes (#1944), `v.remove(i)` of
`if let Some(i) = v.iter().position(..)` was guarded, `$removeOutOfBounds`,
where THIR's ranges had proved it: MIR's didn't take `position`'s `Some`
as below the length. They do now. A slice's iterator records the slice it
steps (`iterates`), through a `&mut` lent of it or a move; its
`position()` or `rposition()` records the `Option` it gives as that
slice's (`positions`), and a `?` of it, `Try::branch`, its `ControlFlow`,
whose `Continue` holds the index. Where the payload is read, `(o as
Some).0` or `(c as Continue).0`, which only that variant's path reads,
the index is below the slice's length (`below`, `Term::Len`), as the
`get(i)` shape takes `get`'s `Some`. What writes the slice between forgets
it, so `items.truncate(1)` between keeps the guard and the remove panics
as Rust's does.

The guide's collections page (`removeNamed`, `moveToEnd`) has its plain
`splice` back, and React's port its noop and test hosts' and
TestSelectors': 11 guards gone, an `$insertAt` a `splice` again. Held by
`test/corpus/position_index_removed_unchecked.rs` (`position`, `rposition`
with `?`, and one shortened between that panics, against native Rust).
Mutations `position-index-not-below`, `slice-iterator-not-lent`
(`lower/analysis/intervals.ts`) and `position-not-through-try`
(`lower/analysis/intervals/calls.ts`).

## M4: THIR's ranges are gone (2026-11-15)

Since step 5's switch (733270bab), THIR's ranges decided nothing: their
four facts, `in_bounds`, `exact`, `set_empty` and `range_casts`, were
read only by the shadow's comparison, and `exact` by one path more, a
`+`, `-` or `*` MIR's intervals don't walk (a trait's `+=`, an atomic's),
which now proves nothing: a wrap kept. So `body_queries/ranges` (3,559
lines), `collect_ranges`, the four facts, `known_in_bounds`, `exact_at`,
`indexes_in_bounds` and what only they read (`the_one_at`,
`constant_integer`, `vec_view`, `cell_view`, `row_view`, `ord_max_or_min`,
`is_iterator_enumerate`, `_rev` and `_step_by`) go. `present_item_of_sized`,
which MIR's hints ask of a `let`-`else`, moved to
`body_queries/present_item.rs`. The shadow, `--intervals-shadow`, lists
each check MIR proves, a `proved-<kind>` line, which `scripts/ablate.ts`
counts as before; test/intervals-shadow.test.ts's expectations are those
lists, and `wrapsDisagreed` is `wrapsProved`, the wraps MIR proves. The
corpus's snapshots and React's port are unchanged. Change 1 of #1681,
#1672's campaign, is done.

## Amendment: a `Vec` that may have a raw pointer kept of it (2026-11-16, #1966)

`called_on` (#1672 M3 step 5, #1946) took a `Vec` lent `&mut` only to
calls as unlent, its length known again from a test past them. Borrowck
holds what such a call does with the `&mut` until the `Vec` is used again,
but not a raw pointer it keeps. Under Stacked Borrows and Tree Borrows
alike, a read through `v` leaves that pointer able to write. So a later
call truncated the `Vec` through it, and `v[k]` under `k < v.len()` read
`undefined` where Rust panics. This is #1782's hole, for a length.

A `Vec` given `&mut` to the crate's code, or to any as a closure that holds
it, is now escaped from then on, until it's assigned again. Its length is
known from a test only until a call that may run the crate's code, a
drop that runs code, or a write through a raw pointer, as a cell's is. A
call may run the crate's code where it's not std's, or is std's given what
carries code: a closure, a `dyn`, a function pointer, a type of the
crate's, a type parameter, or an `fmt::Arguments` where the body's
formatting may run code, an `Argument` made of such a type or one made
elsewhere (`may_write_escaped`, `formats_code`); or one given what may
hold a raw pointer, `ptr::write(p, ..)`, and a `&mut` made through one,
`(*STASH).truncate(0)`'s, which write through it running no crate code.
So `format!` of numbers
and `String + &str` keep the length. Std's code given the
`&mut Vec` itself keeps no pointer to it, but where its signature lets
one out, `ptr::from_mut`'s (`mut_refs`'s `lets_pointers_out_of`, whose
`holds_a_raw_pointer` counts std's `NonNull`). A `push` lends it `&mut`,
which leaves no pointer kept before able to write it, so what a push
shows needs no forgetting. The bounds matrix's holder
`vec_kept` gives the `Vec` to a function that keeps a pointer, and
truncates it through the pointer between the test and the index: 13
programs. Without the rule, 11 of them run otherwise than native Rust;
`in_closure`'s and the loops' `swap`/`drain` are undefined behaviour under
Miri, left out. Corpus case `vec_lent_to_code_that_keeps_a_pointer`;
mutations `called-on-kept-pointer-not-escaped` and
`escaped-kept-past-a-call`.

## Amendment: the crate's code that keeps no pointer, as pointer provenance says (2026-11-16, #1947)

#1966 escaped a `Vec` given `&mut` to any of the crate's code. Whether a
call may keep a raw pointer to what a `&mut` it's given refers to has one
answer already, `pointer_may_keep` (#1781), which walks every body the
call runs: the crate's functions and closures, each trait impl it may
reach, and what it holds. It is now a function of `tcx`, the typing
environment and the crate's libraries, in `pointers/keeps.rs` beside
`returned_from`, and the walk asks it of a call that isn't std's: such a
call escapes the `Vec` only where it may keep one. A std call keeps
#1966's rules, since `pointer_may_keep` takes std's signatures at their
word, and `ptr::from_mut` returns a raw pointer its signature names no
lifetime for. A walk that doesn't lower a body, what callers give or a
closure's result, isn't given the crate's libraries and keeps #1966's
escape. Test/architecture-pointers.test.ts holds that only `pointers/`
answers it.

React's port: 123 `$index` to 117, the Flight loaders'
`exportedEntries[0]`, given to `add_local_exported_names` before
`js::truthy` between the test and the index, a plain index, none added.
The bounds matrix's `vec_kept`, whose `keep` stores a raw pointer, stays
escaped, its 13 programs as native Rust runs them.

## Amendment: the relational part's derivation rules (2026-11-16, #1948, #1949)

M2b-1's `below`, each `x < y` and `x <= y`, gains rules that derive it:
a half at most what it halves, twice what's below a half below the whole
(the one scaled form), and a bound through one integer between; next, a
length shifted, a step-2 loop's index and callers' facts. General
difference bounds, `x ≤ y + c`, come with the first shape needing another
`c` ([1168](1168-difference-bounds-in-the-mir-intervals.md)).

## Amendment: an integer a closure captures, lent only to calls (2026-11-16, #1947)

The Flight loaders' `exportedEntries[nextEntryIdx]` still read `$index`:
`next_entry_idx` is captured `&mut` by a closure given to `read_mappings`
as a `&mut dyn FnMut`. An integer lent `&mut` anywhere was lent away for
the whole body, so the loader's own test of `nextEntryIdx` after the call
bounded nothing.

`called_on` now treats an integer as it treats a `Vec` (#1946, #1977). Its
`&mut` held by a closure, or given `&mut` itself to a call, is lent only
to the calls it's given to, and each of those calls forgets its value
(`called_on_mut`); a fresh test after the call bounds it. Each holder that
is a closure, or holds one (`&mut take as &mut dyn FnMut`), carries that
closure's code, `CalledOn::closures`. A call that may keep a raw pointer
to the integer lends it away from then on, as before: the closure's own
body, `closure_may_keep`; the call given the closure, which runs that code
behind the `dyn`, `pointer_may_keep_behind`, where `pointer_may_keep`
alone fails closed; or, given the `&mut` itself, `pointer_may_keep` of the
call, which fails closed on code it can't follow, a type parameter's
`K::keep`. A holder used any other way, written through or stored, lends
its integer for the whole body, as before.

React's port: 117 `$index` to 114, the three Flight loaders'
`exportedEntries[nextEntryIdx]`, and nothing else. Corpus case
`integer_captured_by_a_closure_given_to_calls` holds the closure, twice
called, given `&mut` itself, kept by the crate's `Keeper::keep`, and kept
by generic code's `K::keep`, against native; each kept one panics as Rust's
does, where its unkept form would read `undefined`.

## Amendment: a guard's fields as locals (2026-11-18, #1815)

The undo stack's `undo` read `history.index = (history.index - 1) >>> 0`
under `if (history.index === 0) return;`: its history is an
`Rc<RefCell<History>>` the stack's closures capture, so each field is read
through a `borrow_mut()` guard's `Deref` or `DerefMut`, a new pointer each
time, and the walk knew a struct's fields only through a reference
parameter (`fields_as_locals`).

`guard_fields_as_locals` makes each field a guard lends a local of its
own, before the walk, as `fields_as_locals` does a parameter's. Every
`Deref` and `DerefMut` of a guard gives the same struct while it lives,
and nothing else reaches it: another borrow of a `RefMut`'s cell panics,
and nothing writes a `Ref`'s. The local is unknown where the guard's made,
as no path before it writes it, so a guard made each turn of a loop knows
nothing of the last turn's, which a call between the turns may have
changed. Not of:
- a guard whose local is made at more than one place, `history =
  second.borrow_mut()`, which is another borrow's past it;
- a guard a reference to which is used past its own `Deref`s,
  `step(&mut history)`, or a pointer of which is used past its fields,
  `history.rewind()`.

A use of the guard itself is a move or a drop, and borrowck ends its
pointers' uses before either, so it isn't counted. A guard `RefMut::map`
consumes isn't a struct's guard, and keeps its checks. A field that holds
a cell, `count: Cell<u32>`, which another `Ref` of the struct may set, is
a local that holds one: the walk knows nothing of what a cell holds but a
length, which `cells` forgets at any call or `borrow_mut()` as it does
any cell's, so no field is left out for its cell (a `Freeze` condition
was measured to decide nothing, its mutation surviving).

Three corpus cases move, each a proof: two undo stacks' `undo`, now
`history.index--`, and `rc_field_of_a_parameter_beside_an_untagged_rc`'s
`entries.remove(0)` right past `entries.push(state)`, in bounds as the
push leaves at least one, now `history.entries.shift()`. React's port
doesn't move.

The undo stack's `undo` is `history.index--`; its other checks would
need the struct's invariant, `index < entries.len()`, across its closures
(#1815's second step). That invariant doesn't hold: `map` gives its
borrow back before calling `update`, which may use the stack, and a
`record` there pushes an entry that `map`'s write of the shorter list
then leaves `index` past, so `redo` panics `index out of bounds: the len
is 1 but the index is 2`, and its checks stay (corpus case
`undo_stack_map_reentered`, as #1815's port writes it). Corpus case
`refcell_guard_fields` holds each case above against native, each kept
check wrapping as Rust does where its unkept form would give -1.

## Amendment (2026-11-18, #1860): a `split` collected is never empty, so its `last()` is `Some`

`path.split('.').collect::<Vec<_>>().last().unwrap()` kept `$unwrap(keys.at(-1))`,
though a `split` gives the whole text where its separator isn't there,
`""` too: what it gives collected is never empty. Two facts of the walk,
and one proof:
- **A `split` made** (`calls.rs`): a `str::split` or `rsplit` call's
  local, in `State::splits`, until it's written or a `&mut` of it is
  lent, as `s.next()` lends one, which forgets it.
- **Collected straight** (`calls.rs`): `Iterator::collect` into a `Vec`
  of such a local, moved there, gives the `Vec` a least of one
  (`State::least`), beside `vec![x; n]`'s. Not a `split_terminator`'s,
  a `split_inclusive`'s or a `splitn(0, ..)`'s, which may give none, as no
  other call makes a `split`. A `clear()` or any `&mut` of the `Vec`
  forgets its least.
- **Proved present** (`proved.rs`, `Proofs::present`): a slice's
  `first()` or `last()` (`SliceLength::End`) of one whose least is one or
  more, as `least_known` says, the least `index_below` reads too, now one
  function. Its `Option` is `Some`: `ended_present` (`mir/nulled.rs`) says
  so, `proved_present` of a variable's read and `field_read_marked` of a
  temporary read in place, `keys.at(-1)`, the two places a read is marked
  proved, so `unwrap()` gives it with no check (#1933's path). Such a call
  is a reason to walk a body (`body_proofs`). Which call makes a `split`
  and which collects into a `Vec` are recognition's, `makes_a_split` and
  `collects_into_vec`.

Case C of ADR 0262: a check left where the `Vec` may be empty keeps
Rust's panic. Corpus case `split_collected_is_never_empty` runs against
native in both builds: `split` and `rsplit` collected, of text with and
without the separator and of `""`; a `split` stepped before its `collect`;
a `Vec` emptied after it, whose `unwrap()` panics as native Rust's, under
`catch_unwind`; and `split_terminator` and `splitn(0, ..)`.
test/lowering-collections.test.ts holds the JS: `return keys.at(-1);`,
`return words[0];`, and `$unwrap(` kept where stepped, emptied or
terminated. Mutations `split-collected-least-unknown`,
`slice-end-present-of-any` and `ended-present-unmarked-in-place` are
caught.

## Amendment (2026-11-18): a guard lent shared keeps its fields locals (#1944's regression)

`733270bab`, MIR's intervals deciding a call's indexes, lost the undo
stack's `remove(0)` past a push: `history.entries.shift()`, as upstream
writes it, became `if (0 >= history.entries.length) $removeOutOfBounds(..);
history.entries.splice(0, 1)`. `record`'s closure, `group.is_some_and(|group|
.. history.open ..)`, captures `&history`, a shared reference to the
`RefMut` guard, and `guard_fields_as_locals` took any use of a reference to
a guard but its own `Deref` for the guard lent whole, `step(&mut guard)`,
so the guard's fields were no locals and the length the push grew was
forgotten.

A shared `&guard` reaches nothing but `&History`, by `Deref`: through it no
field changes but a cell's, and the cell can't be borrowed mutably again
while the guard lives. So a reference lent by a shared borrow, used past its
own `Deref`, isn't a use of the guard whole; a `&mut guard` still is. A
field that holds a cell is a local that holds the cell, as before: the walk
knows nothing of a `Cell`'s value, and a call forgets a `RefCell`'s length.

Corpus case `guard_lent_to_a_closure_proves_remove` holds both against
native: `record`'s shape, whose `remove(0)` is `shift()`, and a closure
given `&history` that changes a `Cell` field through it, `history.at`, whose
index keeps its check and panics as Rust does. Mutations
`guard-fields-of-a-guard-lent-shared-whole` (the proof lost) and
`guard-fields-of-a-guard-lent-mut-as-shared` (the unsound direction, caught
by `refcell_guard_fields`' `step(&mut history)`).

## Amendment (2026-11-18, #2053): a `Vec` lent through a `dyn` escapes where the code keeps a pointer to what the `dyn` gives out

`stash(v: &mut dyn Items)` that keeps `v.items() as *mut Vec<u32>` left
the `Vec` it was given unescaped (#1966's rule). Its length stayed known
past `clear_it()`, which cleared it through the pointer, so `len() - 3`
past `len() > 2` was `-3`, and a later `v[0]` read `undefined` where Rust
panics. The metamorphic net's `dyn-ref` rewrite of
`vec_lent_to_code_that_keeps_a_pointer` found it. `pointer_may_keep`
(`pointers/keeps.rs`) took a raw pointer to the given `&mut`'s pointee
only, `*mut dyn Items`. But behind a `dyn` whose code isn't known, what
its methods give out may be any type: of such a pointee, a raw pointer to
any type may be to it, and a reference to any type given to a call it
can't follow may carry it. That fails closed, as the walk does of what it
can't see. Not of a `dyn` of the `Fn` traits whose closure is known
(#1947's `behind`): there the question is only whether the call keeps a
pointer to the closure, and what the closure keeps of its captures is
`closure_may_keep`'s, asked apart. Widened there too, it added two checks
to `integer_captured_by_a_closure_given_to_calls` that nothing needs.

Case C of ADR 0262. Corpus case
`vec_lent_through_a_dyn_to_code_that_keeps_a_pointer` holds a pointer kept
to what the `dyn` gives out, one kept to the `dyn` itself, each cleared
past a length test, and an index past it that panics, against native.
Main gives `-3` and no panic. Mutation `dyn-pointee-keeps-only-its-own-type`
is caught.
