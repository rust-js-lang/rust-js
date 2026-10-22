# 0577. A function only calls reach is what its calls give it; a `push` and a `pop` change a length by one

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md),
whose ranges stopped at a function's parameters.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. An index left
unchecked is one Rust's check would pass, and a wrap left out one that
would change nothing, as in ADR 0576.

## Context

ADR 0576 proved React's heap's `siftDown` and `pop`, but not `siftUp`:

```rust
pub fn push<T: Node>(heap: &mut Heap<T>, node: T) {
    let index = heap.len();
    heap.push(node);
    sift_up(heap, index);
}
fn sift_up<T: Node>(heap: &mut Heap<T>, i: usize) { .. heap[index] .. }
```

`i` is a parameter, any `usize` where the body is all that's read, so
`heap[parentIndex]` and `heap[index]` were `$index` and the swap `$swap`.
Its one caller says more: `i` is the heap's length before a `push`,
below its length after.

## Decision

**A function only calls reach is given what its calls give it**
(`analysis/only_called.rs`, `body_queries/ranges.rs`). One no JS caller
sees, not exported, not a trait's method, and never named as a value,
whose every call in the crate's code is in a body rust-js walks, is
walked from what each call, where it's reached, tells it of its
parameters: each integer's range, its bound by the slice another
parameter is given, and each slice's least length. Those are joined, and
each function told otherwise is walked again until what it's told holds;
what keeps changing is forgotten, as a loop's is (ADR 0576). One no call
reaches keeps every check.

**A `Vec`'s `push` makes its length one more, and `pop` one fewer where it
has one**: a bound `⌊(c·L + b) / 2^s⌋` by its length is `⌊(c·L' + b -
c·by) / 2^s⌋` by the new one, and its least length moves by one. Before,
either forgot all that was known of it.

**A bound is kept over its least denominator**, `⌊(4·L - 4) / 4⌋` as `L
- 1`: `siftUp` halves its index each turn, and a bound that doubled its
denominator each time never settled.

```js
function siftUp(heap, i) {
  let index = i;
  while (index > 0) {
    const parentIndex = (index - 1) >>> 1;
    if (compare(heap[parentIndex], heap[index]) > 0) {
      [heap[parentIndex], heap[index]] = [heap[index], heap[parentIndex]];
```

## Why

- **What the program shows, carried where it goes**: through a call, as
  through a loop. A private function's callers are all of its callers.
- **One question, one owner**: which functions only calls reach is
  `only_called`, read of the crate's HIR, every body's, so a call in what
  rust-js doesn't walk, a derive's or an array length's, is counted and
  keeps its function's checks.

## Alternatives

- **Inline a private function's checks into its callers**: JS no one
  writes. Rejected.
- **An attribute that says an index is in bounds**: what rustc can't
  check, and a second way to say what the program already says. Rejected.
- **Trust every private function's callers, named as a value or not**: a
  callback is called with anything. Rejected.

## Consequences

- React's heap's JS imports no helper: every index and swap reads as
  upstream's.
- 15 corpus cases lose a wrap of a parameter their callers bound.
- A body is walked once more each time what it's told grows: a crate's
  walk is a fixpoint, not a pass, bounded by the same forgetting as a
  loop's.

## What holds it

- `test/lowering.test.ts`, "a function only calls reach is what its
  calls give it": `siftUp` of `push`, an index its one caller tested, one
  called where nothing bounds it, a function named as a value, one a JS
  caller sees, a recursive one, `push` and `pop`, run in Node.
- Mutations: a value's name not counted, an exported function trusted, a
  call that tells nothing, a bound not mapped to its parameter, a `pop`
  taken as a `push`, a bound never lowest.

## Amendment: no constant only rustc evaluates

The analysis typechecked every body's owner, a const argument in a
signature among them, `FixedOutput<'_, 10>`'s `10` in a trait nothing
implements, which rustc can't typecheck as a body before it lowers it:
four of rustc's const-generics tests crashed, `type_of` called on an anon
const too early. It walks no constant only rustc evaluates
(`analysis::is_type_system_const`, which `inline_consts` asks too): JS
never runs one, so no call in it reaches a function.
`test/corpus/const_argument_in_a_signature.rs` holds it; mutation
`only-called-walks-type-system-consts`.
