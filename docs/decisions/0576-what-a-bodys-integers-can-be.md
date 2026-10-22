# 0576. What a body's integers can be: an index in bounds, an operator that can't wrap, a swap JS's own

Status: Accepted. Replaces [0292](0292-known-bounds.md)'s
regions, whose cases it keeps, and narrows where [0011](0011-numbers.md)'s
wraps are written.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for each. Nothing a
program does changes: an index left unchecked is one Rust's check would
pass, and a wrap left out is one that would change nothing.

## Context

React's scheduler heap, ported (ADRs 0574, 0575), read upstream's
arithmetic with every check and wrap rust-js writes:

```js
// upstream's siftDown:                 // rust-js's:
const leftIndex = (index + 1) * 2 - 1;  const leftIndex = (Math.imul((index + 1) >>> 0, 2) - 1) >>> 0;
const left = heap[leftIndex];           if (compare($index(heap, leftIndex), $index(heap, index)) < 0) {
heap[index] = right;                      $swap(heap, index, rightIndex);
```

Each is right where nothing is known: a `usize` is a `u32`, whose `+`
wraps (ADR 0011), and `v[i]` panics past the end (ADR 0292). Here each is
known. `index < halfLength`, `halfLength` is `length >> 1` and `length` is
`heap.len()`, so `(index + 1) * 2 - 1` is below the length, can't pass
`u32`'s largest, and `heap[leftIndex]` is in bounds; `rightIndex < length`
is tested before `heap[rightIndex]`. ADR 0292 knew only a condition's
region, of a variable the body never sets again: the heap's `index` is set
each turn.

## Decision

**A body's integers have ranges, found by walking it as it runs**
(`body_queries/ranges.rs`): each integer variable's least and greatest,
and its bound by a slice's length, `⌊(c·L + b) / 2^s⌋`, `L / 2 - 1` of
`index` above. A condition narrows them where it holds, its `if`'s branch,
past an `if` whose other branch leaves, the right of an `&&`; an operator
gives its operands' range; a loop is walked until what's known at its
start holds, and what still changes past a few rounds is forgotten.
What may change what's known forgets it: a variable set, a `&mut` lent or
kept, a `Vec` given by `&mut` to what may change its length, a closure
made that captures by `&mut`; a raw pointer's variable is never known.

Of the ranges:

- **An index below its slice's length is in bounds**, `heap[leftIndex]`:
  each `a[i]`, a `Vec`'s `Index::index`, and `swap`'s and `swap_remove`'s,
  every time it's reached. A slice's length is a `Vec`'s through `deref`,
  and a `for i in 0..xs.len()` is below it where its body never changes
  `xs`.
- **A `+`, `-` or `*` whose range is its type's is JS's own**, `(index +
  1) * 2 - 1`: of 32 bits or fewer, where it's written outside a macro.
  `binary` writes it without its wrap, and `x += 1` reads as written.
- **A `swap` in bounds is JS's swap**, `[heap[i], heap[j]] = [heap[j],
  heap[i]]`, and a `swap_remove(i)` in bounds the last swapped in, then
  popped: `const first = heap.pop();` (`mir/bounds.rs`, with the `Assert`
  and `v[i]` the ranges know).

Where the walk can't follow a jump, a `#[loop_match]` or `asm!`, nothing
it found is trusted.

## Why

- **It's what the program shows**, as ADR 0292's conditions are, carried
  where the program carries them: through a loop that sets its index each
  turn, through arithmetic, past the `if` that tested it.
- **The JS a person writes of it**: upstream's heap, `n += 1`, `fib(n -
  1)` of `n >= 2`, `items[items.length - 1]` of one that isn't empty.
- **One analysis answers both**: an index is in bounds and an operator
  in range by the same ranges, each asked where it's lowered, not two
  analyses of one body that could disagree.

## Alternatives

- **Keep ADR 0292's regions and add arithmetic beside them**: two answers
  to what an index is, and the heap's `index`, set each turn, still
  unknown. Rejected.
- **Leave out every wrap of a `usize`**, as if it were JS's number: a
  `0 - 1` would be `-1`, not `4294967295`. Results change: rejected.
- **`$swap` and `$swapRemove` unchecked where in bounds**: still a helper
  no one writes. Rejected for the swap JS has.

## Consequences

- 17 corpus cases and 8 snapshots lose a wrap or a check, as React's
  heap's `siftDown` and `pop` read as upstream's but for its `node`
  parameter; the readability baseline is six lines shorter.
- Tests that held a wrap or a check now known to be needless hold the
  form without it.
- What's known stops at the body: a function's parameter is any value,
  so the heap's `siftUp`, whose index is its caller's, keeps `$index` and
  `$swap`.

## What holds it

- `test/lowering.test.ts`, "a body's integer ranges show indexes in
  bounds and operators that can't wrap": the heap's `siftDown` and `pop`,
  wraps of what could pass its range kept, a slice changed in a loop, an
  index nothing bounds, a length changed, run in Node; and ADR 0292's "an
  index a condition shows in bounds is read as it is".
- `test/architecture.test.ts`: `mir_std_call`'s cap lowered, the in-bounds
  calls moved to `mir/bounds.rs`.
- Mutations: each condition's narrowing undone or loosened, a `return`, a
  `continue` or a `break` not followed, a loop's turns unread, a length
  kept where it changed, a wrap's bound unchecked, a shift unscaled, an
  index at most the length taken below it, a swap unjudged or unswapped,
  the in-bounds call checked, an exact operator wrapped.
