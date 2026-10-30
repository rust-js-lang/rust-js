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
