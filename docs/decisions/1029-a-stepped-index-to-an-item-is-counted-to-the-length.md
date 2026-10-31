# 1029. A stepped index to an item is counted to the length

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: each turn steps the index, tests it against the slice's
length as it is then, and runs the body, in that order, as Rust's does.

## Context

React DOM's inline Fizz runtime (ReactDOMFizzInstructionSetShared)
completes its boundaries in pairs, reading a batch that grows as it runs:

```js
for (let i = 0; i < batch.length; i += 2) {
  const suspenseIdNode = batch[i];
  const contentNode = batch[i + 1];
  ..
}
```

Its port is the Rust that reads the length each turn too, `for i in
(0..).step_by(2) { let Some(&id) = batch.borrow().get(i) else { break };
.. }` (rd260), which printed

```js
for (const i of $stepBy($iterator({ start: 0 }, $rangeFromStep(0, 4294967295)), 2)) {
  const id = batch[i];
  if (id == null) {
    break;
  }
  ..
}
```

and `$stepBy` and the range's steps bring in the runtime's BigInt code,
which Closure, that compiles the inline runtime to ES5, can't transpile:
two of React's inline entries didn't build.

## Decision

**A `for` of `(s..).step_by(n)` whose body starts with `let Some(x) =
v.get(i) else { break; }`, of items that take up room, is `for (let i = s;
i < v.length; i += n) { const x = v[i]; .. }`** (`i++` of a step of 1).

- `body_queries::present_item_of_sized` (ranges' owner of an index below a
  length, ADR 0881) says a `let`'s `Some` pattern of `v.get(i)`,
  `.copied()` too, of items that take up room, is there exactly where `i`
  is below the length; `hints::Written::present_breaks` records each such
  `let … else { break; }`.
- `iterators::stepped_from` reads `(s..).step_by(n)`'s made iterator, a
  whole step from 1 up.
- Tidy's `counted to lengths` pass (`loops::counted_to_lengths`, in
  `names`, after `named temporaries` and before `range steps`) makes the
  `for` of a `for…of` of it whose body is that `let`'s `const x = v[i];
  if (x == null) { break; }`, the index the loop's own, `v` a variable or
  a path of fields, which the test reads each turn as `get` does.

Why it's the same:

- **Order**: Rust's turn steps `i`, then `get(i)` reads the length and
  breaks past it, then the body; the `for`'s step, test and body are that.
- **The length each turn**: an item the body pushes is visited, as it is
  in Rust (the corpus' `grown`).
- **No overflow**: where `get(i)` holds, `i` is below the length, at most
  `isize`'s most of items that take up room (ADR 0701), so a step of up to
  `2^31` can't pass `usize`'s most, where Rust's would panic; a larger
  step, or items of no size, keep the iterator.
- **Each turn's `i`**: a `for`'s `let` is each turn's own, as Rust's
  binding is, so a closure keeps its turn's (the corpus' `kept`).

Not of an item that may be `None` itself, `$someAt` (ADR 0881), nor of
`get(i).copied().flatten()`, whose `None` an item gives too: the fact is
the `let`'s, not the JS's shape.

## Alternatives

- **Keep the iterator**: correct, but no JS reads so, and it's the code
  Closure can't make ES5 of.
- **Write `while i < v.len()` in the port**: a counter the program steps,
  what clippy's `needless_range_loop` and the Rust book steer from; the
  port's Rust is the idiomatic form.

## Consequences

- Held by `test/lowering-written-loops.test.ts`' "a stepped index to a
  slice's item is counted to its length", corpus `stepped_index_to_length`
  against native Rust (pairs, a batch grown while it runs, `None` items,
  `flatten()`, items of no size, closures of each turn's index), the
  guide's `completed`, and mutations `counted-to-lengths-unmade`,
  `counted-break-of-any-none`, `counted-of-another-index` and
  `counted-step-past-most` (`mir/tidy/loops.ts`).
- Left: `for i in 0..` with the same `let`, a `while` of `$rangeFromNext`,
  and `(0..v.len()).step_by(n)`, `$lazyRange(0, v.length, n)` read by
  `$index`, are not counting `for`s yet.

## Amendment: an item that's a reference is a pointer's size (2026-10-31)

React's own batch, the port's `RefCell<Vec<&Element>>`, still printed
`for (const i of $stepBy($iterator({ start: 0 }, $rangeFromStep(..)),
2))`: `present_item_of_sized` took the item of `get`'s `Option<&&Element>`
by peeling every reference, `Element`, which has no size in Rust (a JS
value's type), so the item was taken to be of no size, which a slice may
hold more of than `usize`'s most.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above.

- **The item is `get`'s `Option<&T>`'s `T`, one reference peeled**, of
  `copied()`'s `Option<T>` too, which leaves out only a reference to what
  has no size: `&Element` is a pointer's size, whatever it refers to, so
  `for (let i = 0; i < batch.length; i += 2) { const suspenseIdNode =
  batch[i]; .. }`, as upstream.

Held by `test/lowering-written-loops.test.ts`' "a stepped index to a slice
of references is counted to its length" and mutation
`present-item-peeled-whole` (`body_queries/ranges/present.ts`).

## Amendment: a range's `step_by(n)` is a counting `for`, and an item that may be `None` is counted too (2026-10-31)

Two of the near-misses above:

- **`for i in (a..b).step_by(n)`**, `(0..v.len()).step_by(2)` say,
  printed `for (const i of $lazyRange(0, v.length, 2)) { .. $index(v, i)
  .. }`. It's now the counting `for` `a..b`'s is (`loops::counted_range`,
  the one maker of a range's `for`), `for (let i = 0; i < v.length; i +=
  2)`, its step `i += n` of a whole `n` from 1 to `2^31`
  (`iterators::stepped_by`), its end read each turn where it's the same
  then, held in a `const end` first otherwise, as `a..b`'s; of `a..=b`
  `i <= b`. Not of a range stepped from its end, `(a..b).rev().step_by(n)`,
  which keeps `$lazyRangeBack`. And the ranges' owner (ADR 0881) reads a
  range's `step_by(n)` as the range for what its items are
  (`ranges::walk_for`): each in it, so below the slice's length its end
  is, `v[i]`.
- **An item that may be `None` itself**, `v.get(i)` of a `[Option<T>]`,
  boxed by `$someAt` where it's got: `const got = $someAt(v, i); if (got
  == null) { break; } const x = $someValue(got);` is counted to the
  length too, `const x = v[i];`, the item as it is, a `None` too. The
  fact is the `let`'s, which holds of such an item as of any.

Why it's the same: JS's numbers don't wrap where `i += n` passes the end,
as Rust's `StepBy` doesn't step past it either; the step is a constant,
so no `step_by(0)` panic is left out (a step of `0` keeps the iterator,
which panics as Rust's does).

Case ([0262](0262-when-rust-and-js-disagree.md)): N.

Held by the corpus case `stepped_range_loops` (pairs, a slice changed
while it's stepped, `a..=b`, a step of 1 and each turn's closure, a range
stepped from its end, items that may be `None`) and the mutations
`stepped-range-for-unmade`, `stepped-range-back-counted-up` and
`counted-some-at-unread` (`mir/tidy/loops.ts`) and
`stepped-range-index-unknown` (`body_queries/ranges.ts`).

Left: `for i in 0..` with the same `let`, a `while` of `$rangeFromNext`.
