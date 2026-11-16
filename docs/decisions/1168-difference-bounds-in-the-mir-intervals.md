# 1168. Difference bounds in the MIR intervals

Status: Accepted (2026-11-16, the lead's decision; #1948, #1949). It
extends [1137](1137-the-bounds-analysis-on-mir-shadow-first.md)'s
relational part (M2b-1's `below`), which 1137 points here for.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about what
the bounds analysis proves. A check it proves away is one Rust's can't
fail. One it can't prove stays, as `$index`.

## Context

Since #1672's switch (step 5, #1944), the MIR intervals decide each
call's indexes. Their relational part is small by design (M2b-1): a set
`below` of `x < y` and `x <= y`, `y` a slice's length or an integer
local, and each local that holds a length. THIR's ranges, which decided
before, proved shapes this can't. Each shape kept a check, and each test
whose expectation changed names #1948 or #1949:

- **A length shifted**: `n = v.len(); v.push(7); v[n]`, and `v[n - 1]`
  (`lowering-functions`); `properties[prev_length - 1]` of a length held
  in a variable, tested again (`lowering-integer-ranges`, React's
  performance track). Neither `n < len` nor `n - 1 < len` is a fact of
  `below`: each is `n ≤ len + c` for a constant `c`.
- **A loop stepped by 2**: `batch[i]` of `for i in (0..end).step_by(2)`,
  `end = batch.len()` held (`lowering-cell-flow`).
- **A relation callers give**: `sift_up(heap, heap.len() - 1)`, whose
  `heap[parent]` and `heap[index]` are checked (#1948, React's
  `SchedulerMinHeap`). `callers.rs` passes `below` relations between
  arguments, but a caller's `heap.len() - 1` is no `below` fact.
- **The heap's halving**: `sift_down`'s `heap[left_index]` and
  `heap[index]` under `index < length >> 1`, `left_index = (index + 1) * 2
  - 1` (`lowering-integer-ranges`). This needs `index < length >> 1` to
  give `2·index + 1 < length`.
- **A table's rows** as long as its row count, `board[r][c]`
  (`lowering-collections`). This is the length of a place, `board[r]`, not
  a local's. It belongs to `tables.rs`'s facts, not to this relational
  part (see Not here).

## Decision

**The relations stay `below`'s two kinds, `x < y` and `x <= y`, and the
analysis gains the rules that derive them for each shape** (the lead's
decision): no shape here needs more. Each is a difference bound,
`x ≤ y + c` with `c` of `-1` or `0`. **The swap to general difference
bounds, `x ≤ y + c` of any `c`, comes with the first shape that needs
another `c`**; until then it would be unused generality.

- **Never of what wraps.** rust-js compiles as Rust's release profile does
  (`overflow-checks = off`, `src/main.rs`), where `+`, `-` and `*` wrap,
  so no operation in its MIR is checked. A rule derives a fact only where
  the intervals prove the operation doesn't wrap, the walk's `exact`, or
  where the fact itself bounds it. A `wrapping_*` or `overflowing_*` call
  is a call, never a `BinOp`, and derives nothing; a corpus row holds a
  `wrapping_add` whose index keeps its check.
- **A half.** `h = n >> c` or `n / c` of what's never negative is at most
  `n`, `n` a slice's length or an integer: `half_length <= length`. Of
  `n >> 1` or `n / 2`, `h` is recorded a half of `n` (`State::halves`).
- **The one scaled form: twice what's below a half.** `b = a * 2` or
  `2 * a` of `a <= h`, `h` a half of `n`, is at most `n`, and of `a < h`
  below it. It never wraps, being at most `n`. Of `a < h` it's at most
  `n - 2` (`State::doubled`), so `b + 1` is below `n`: a heap's
  `(index + 1) * 2 - 1`, through the existing `t = x + 1 <= y` of `x < y`
  and `t = i - 1` of `i <= len`, and its `2 * index + 1`. A second scaled
  shape, if one is found, makes the case for general `a·x + b` terms,
  decided then.
- **Through one integer.** A bounds check is proved by `i < m` and `m <=
  len(s)`, or `i <= m` and `m < len(s)`, one integer between: `index <
  half_length <= length`.
- **Next, by the same rules** (each its own patch, below): a `push`'s
  length past a held one, `n < len` after `n = v.len(); v.push(x)`; a held
  `prev_length - 1` retested; a step-2 loop's index under its held end; and
  the facts callers give between arguments, which `callers.rs` passes as
  `below` already.

## Not here

A table's row length, `board[r][c]`, is a length of a place, not a local.
`tables.rs` keeps the row types no `&mut` resizes (`rows_kept`). Proving
`c < board[r].len()` from a row count is its question, filed as #1997.

## Proof

Each shape is proved before the next, in this order, each one patch:

1. The heap (`sift_down`, `sift_up` with its callers), the scaled form
   and the callers' facts.
2. A length shifted, a `push`'s and a held one's.
3. The step-2 loop.

Each patch carries:
- the row's test, its expectation back to a plain index;
- a corpus case per shape against native, with negatives that must stay
  checked and panic as Rust's do: `v[len]`, `v[i + 1]` under `i < len`, a
  step-2 index past the end, and a **wrapping add** whose index stays
  checked;
- the bounds matrix and the twin net;
- React's port's diff, listed to the lead before it lands
  (`SchedulerMinHeap`, the performance track, FlightClient, DOM config,
  the async action);
- a mutation of each source of facts, of the join, of the widening, of
  the checked-only rule and of the scaled form.

## Consequences

- `below` stays; `State` gains `halves` and `doubled`, joined and
  forgotten as its other facts are.
- The heap, step 1, landed with: `sift_down`'s `heap[left_index]` and
  `heap[index]` plain, `2 * index + 1` under `index < len / 2` too;
  `n - n / 2` no longer wraps (`read_to_end_capacity_of_std_readers`);
  React's port's `SchedulerMinHeap` `compare(heap[leftIndex], ..)`, its one
  line. Corpus case `heap_halves_bound_their_children` holds them against
  native, with rows that keep their checks and panic as Rust's: a loop to
  `index <= half`, a right child `* 2 + 2` under the left's bound, and a
  wrapping add. Mutations `half-not-at-most-whole`, `half-unrecorded`,
  `doubled-half-unbounded`, `doubled-below-half-unrecorded`,
  `doubled-plus-one-unbounded` and `below-through-an-integer-unfollowed`
  are caught, and `written-pure-test-bound` is back (#1948 noted it would
  be): the heap's reads are pure again.
- Precision mutation `intervals-callers-relations-unpassed` keeps its
  meaning.
- A length shifted, step 2, landed with two rules. **A `push`'s length is
  one more**, and never wraps, so what was at most the length before it,
  a local holding it or one known `<=` it, is below the length past it
  (`calls.rs`, as `pushed` keeps it: of a `Vec` lent whole, no raw pointer
  made of it). **`x == y` gives each at most the other**, `prev_length ==
  properties.len()`, so the existing `t = i - 1` of `i <= len` puts
  `prev_length - 1` below it, where `prev_length` is at least one. With
  them `v[n]` past `n = v.len(); v.push(x)`, the heap's `sift_up` given
  `index = heap.len()` before its push (as `callers.rs` passes it), a
  table's `board[r]` under `r <= height` past a pushed row, and the outer
  `v[i]` of `v[i] += { let x = v[i]; v.push(0); x }` past its right side's
  checked `v[i]` read plain; React's port's `siftUp` reads and
  `properties[prevLength - 1]`. `v[n + 1]` past two pushes keeps its check:
  two below is no `c` of `-1` or `0`. Corpus case
  `length_shifted_past_a_push` and the bounds matrix's shape
  `pushed_length` hold them against native, with rows that keep their
  checks and panic as Rust's: `n - 1` of an empty vector, a length past a
  `pop()`, a `clear()` or a new vector, another vector's push, an index
  below another slice's length, a wrapping add, an `==` that may be 0 and
  an index at the length itself. Mutations `pushed-held-length-unrelated`,
  `pushed-at-most-unread`, `pushed-held-length-unread`,
  `pushed-below-of-any-length`, `equal-length-unrelated` and
  `equal-length-below` are caught.
- The step-2 loop, step 3, needed no relation. `step_by(2)` of a range was
  proved already, of a slice; what kept `batch[i]` checked was the cell:
  a `for` loop's `next()` was a call that may write a cell, so each turn
  forgot the `RefCell`'s length. A step the walk knows (`range_step`: a
  range of integers' `into_iter()` and `next()`, its `rev()`, `step_by()`
  and `skip()`, a slice's `iter()` and `enumerate()`) is std's code over
  integers or a slice's iterator, which runs none of the crate's and writes
  no cell, so `may_write_cells` reads it as it does a `borrow()` or a
  length (`cells.rs`). With it `batch[i]` of `i` in `(0..batch.borrow().
  len()).step_by(2)` reads plain, the length held or the range's own; its
  `for` reads `batch.length` each turn, as of a slice the body only reads,
  the range's own borrow held the whole loop letting nothing change it;
  and `refcell_drained_in_a_loop`'s `v.borrow()[j]`. React's port has no
  such loop. Corpus case `cell_length_across_a_loops_steps` and the bounds
  matrix's shape `cell_steps` hold them against native, with rows that
  keep their checks and panic as Rust's: `0..=len`, `i + 1`, a held
  length past a `clear()`, a push or a crate call that empties the cell,
  and the range's own borrow that a `borrow_mut()` in the body panics at.
  Mutations `range-step-writes-cells` and `any-call-reads-cells` are
  caught. The callers' facts need no step of their own: `callers.rs`
  passes `index < heap.len()` of `sift_up(heap, index)` given the length
  before a push (step 2), or `heap.len() - 1` past one, which the `push`
  makes at least one and `t = i - 1` of `i <= len` puts below it.
