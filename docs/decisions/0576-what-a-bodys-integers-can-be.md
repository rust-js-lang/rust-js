# 0576. What a body's integers can be: an index in bounds, an operator that can't wrap, a swap JS's own

Status: Accepted; extended by [0720](0720-a-labeled-block-and-a-named-constant-in-a-bodys-ranges.md),
which follows a labeled block's `break` and reads a named constant's value; by [0577](0577-a-function-only-calls-reach.md),
whose functions are given what their calls give them, and by
[0632](0632-a-js-strings-length-is-below-u32-max.md), a JS string's
`length`, `i + k` tested and a growing counter, whose bound
[0682](0682-a-js-strings-length-is-at-most-int32-max.md) lowers so two
lengths add without a wrap. Replaces [0292](0292-known-bounds.md)'s
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
- What's known stopped at the body, so the heap's `siftUp`, whose index
  is its caller's, kept `$index` and `$swap`, until a function only calls
  reach was given what they give it
  ([0577](0577-a-function-only-calls-reach.md)).

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

## Amendment: a product that can be `-0` isn't exact

A product in range was JS's own `*`, but in range isn't enough: `-4 * 0`
is JS's `-0`, which an integer never is ([0064](0064-numbers.md)), and an
`f64` made of it, `{:e}` of it and `1.0` over it showed it (#108). A
product is exact only where its ranges show it can't be one of them 0 and
the other below 0 (`Range::may_make_negative_zero`); one that can is
wrapped, `n * 0 | 0`, and `Math.imul` of variables, whose result is
never `-0`. ADR 0262's case: results would change, so Rust's, paid for in
output. `saturating_neg` of 0 had the same `-0`, `Math.min(-0, MAX)`, and
is `| 0` as `saturating_mul` is.

Held by `test/corpus/product_sign.rs` and the mutations
`product-negative-zero-exact` and `saturating-neg-negative-zero`.

## Amendment: a bound by another integer's value (#335)

trekhleb/javascript-algorithms' `pascalTriangle`, ported, read
`((((lineNumber - numIndex) >>> 0) + 1) >>> 0)` where upstream's is
`lineNumber - numIndex + 1`: `numIndex < currentLineSize`, and
`currentLineSize` is `lineNumber + 1`, so `numIndex ≤ lineNumber`. A bound
was by a slice's length alone. **An integer is bound by another's value
as by a length**, `([VALUE], n)` for `⌊(c·n + b) / 2^s⌋`
(`known::VALUE`):

- `for k in a..n`, or a test `k < n`, puts `k` at most `n - 1`, where
  nothing else bounds it; a slice's length first, which an index is
  judged by.
- `n + k`, `n - k`, of a variable `n` bound by nothing else, are at most
  `n + k`, `n - k`; a sum of an operand not below 0 that may wrap is
  still at most its bound, as what wraps is less: `line + 1`.
- **`n - k` of a `k` at most `n + b` is at least `-b`**: `n - k - 1` of a
  `k < n` can't wrap, `n - k` of a `k ≤ n + 1` can.

The base is an unsigned integer of 32 bits or fewer (`value_base`), never
below 0 nor past `u32::MAX`, as a length isn't. A bound by `n` is
forgotten where `n` changes (`Known::changed`), and one by the value a
variable is set from, `n -= 1`'s by the `n` it was, isn't kept for the
new. Joined, a range keeps a bound by a value only where both have it:
a counter of no bound joined with one below `n` would otherwise be at
most `n`, then `n + 1`, never settling round its loop. ADR 0262's case A:
a wrap left out is one that changes nothing.

The narrowing of a condition, `refine` and `narrow`, moved to
`ranges/narrowing.rs`, with its mutations. Held by
`test/corpus/bounds_by_a_value.rs`, `lowering-numbers.test.ts`'s "an
integer below another's value doesn't wrap", and the mutations
`value-difference-unbounded`, `value-difference-past-bound`,
`value-wrapped-sum-forgets`, `value-assigned-keeps-own-bound`,
`value-range-end-unbounded`, `value-test-unbounded`,
`value-bound-over-length` and `value-joined-with-none`. Not yet: an index
below a length that grows each turn with its counter, `currentLine[numIndex
- 1]` of a `push` a turn, which needs the two related round the loop.

## Amendment: a loop over an inclusive range (#337)

A `for` over `a..b` gave its variable `a.lo ..= b.hi - 1`, but one over
`a..=b`, `RangeInclusive::new(a, b)`, nothing: integerPartition's
`summand_index - 1` of `for summand_index in 1..=number` kept its wrap
where no test showed it at least 1. **`a..=b`'s variable is `a.lo ..=
b.hi`, at most `b` by its bound**, as `a..b`'s is at most `b - 1`. Held
by `lowering-numbers.test.ts`'s "an integer below another's value doesn't
wrap" and the mutations `inclusive-for-unranged` and
`inclusive-for-past-its-end`.
