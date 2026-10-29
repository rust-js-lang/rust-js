# 0881. A `RefCell`'s `Vec` is indexed by its tests, and a drain in bounds is `splice`

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md)'s ranges and
[0806](0806-a-slices-range-in-bounds-is-js-slice.md)'s ranges in bounds
to a `Vec` a `RefCell` holds, to what an index shows, and to `drain`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: an index or a range is unchecked only where Rust's check
can't fail, so neither panics; where the body can't show it, the check
stays, with Rust's message.

## Context

React's ReactAct flushes its queue, a `RefCell<Vec<RendererTask>>` in
its port:

```js
for (; i < queue.length; i++) {
  let callback = queue[i];
  ..
      queue[i] = callback;
      queue.splice(0, i);
      return;
  ..
} catch (error) {
  queue.splice(0, i + 1);
```

The port printed `$index(queue, i)` right after `i < queue.length`;
`const borrowMut = queue; const index = i; borrowMut[$at(borrowMut,
index)] = callback;`; and `$iter($drain(queue, 0, i))`. A `Vec` behind a
`RefCell` had no length the ranges knew: `queue.borrow().len()` is a
guard's `deref`, which the walk didn't follow.

## Decision

- **A `RefCell`'s `Vec` is a slice by the cell**: `queue.borrow()`,
  `borrow_mut()` and a guard's `deref`/`deref_mut` reach what it holds,
  `([CELL], queue)`; a guard held in a variable is a slice of the
  variable. Its test, `i < queue.borrow().len()`, reads without changing
  anything, as `i < xs.len()` does.
- **What may change it forgets it**: another reference to the cell may,
  so any call but one that only reads or views it (a cell's `borrow()`,
  a guard's `deref`, a `Vec`'s view, `len()`, `is_empty()`, an index, a
  swap), a scope that drops a value whose drop may run code (a binding, a
  statement's value, a call's, a struct's, a closure's), and an
  assignment whose old value's drop may (but of the cell whose guard is
  held while it's dropped). A `&mut` through a guard changes the cell's
  variable, as a `&mut` of a variable does. A `bool` holding a test of
  one isn't held.
- **An index given back is below the length**: after `v[i]` gives its
  item, `i < v.len()`, as it panics otherwise.
- **A `drain(a..b)` in bounds is `v.splice(a, b - a)`**, `v.splice(0, i)`
  of `0..i`, and a discarded iterator of an array made here is its
  making, `queue.splice(0, i);` not `$iter(..)`. Out of what the body
  shows, `$drain(queue, 0, i + 1)`, which panics as std does.
- **An item's place written at once keeps its index**: `queue[i] =
  callback`, of MIR's `(*_r) = move x` right after `index_mut`, and the
  `&mut` used nowhere else: nothing runs between to change `i`.

## Why

- ReactAct's loop is the shape of every queue a `RefCell` holds; its
  test and its read have nothing between them.
- **Correctness first**: each way a cell can change between the test and
  the read is a reason to forget, so an unchecked index never reads
  `undefined` where Rust panics; `refcell_vec_bounds` runs each against
  native Rust.

## Alternatives

- **Every index through a cell checked**, as before: ReactAct's `$index`.
- **A cell's length forgotten at every statement**: ReactAct's
  `queue[i] = callback; queue.splice(0, i)` needs the index's length
  past its statement.

## Consequences

- `queue[i]` below `while (i < queue.length)`, `queue[i] = item;
  queue.splice(0, i);` in a loop that runs nothing between, and after a
  call `queue[$at(queue, i)] = 0; queue.splice(0, i);`.
- Elsewhere, an index read shows a later one in bounds: React's
  scheduler's `siftDown` swaps `[heap[index], heap[left]] = ..` after
  reading `heap[left]`.
- Tests: the corpus's `refcell_vec_bounds` (a loop's read, a set and a
  drain, a task's set whose old one's drop runs, and a call, a binding's
  drop, an assignment's drop, a guard's `&mut` and a held test, each
  between, each panicking as Rust does); `lowering-collections`'s item
  place held while its index changes. Mutations: each rule left out.
  The check that the `&mut` an index gives is used once is defense MIR
  never needs (a `let` reborrows it), so no mutation reaches it; nor
  does one reach a statement's value dropped, `x;`, which rustc warns
  of.

## Amendment: an index an item is there at counts on without its wrap

React's ReactPostPaintCallback flushes its callbacks, `for (let i = 0; i <
callbacks.length; i++) { callbacks[i](endTime); }`; its port, `while let
Some(callback) = CALLBACKS.with_borrow(|c| c.get(i).copied()) {
callback(end_time); i += 1; }`, printed `i = (i + 1) >>> 0`, a `usize`'s
wrap. **Where `let Some(x) = v.get(i)` holds, `i` is an index of a slice,
below its length, which is at most `usize`'s most, so `i + 1` can't
wrap**: `i++` (`ranges/present.rs`, of the test's `Let`). Through
`Option`'s `copied()` and `cloned()`, and of a thread-local's
`with_borrow(|v| v.get(i))`, its closure's body, which the crate's bodies
walked have (`recognition::present_item`, `is_local_borrow`). Not where
it doesn't hold, `else { i + 1 }`, where `i` may be any. Case N: the
wrap can't happen. `present_index_counts.rs` runs it against native Rust,
a callback scheduled while they run among them.

What stays: the loop is the `while` the program wrote, `while ((callback
= callbacks[i]))`. Its `for (..; i < callbacks.length; i++) {
callbacks[i](endTime); }` was not taken: it calls each callback as a
method of the array, `this` the array, which a JS function given as a
callback sees, where Rust's call gives it none; and it's a second form of
a `while` (0879). Nor is its `let callback` renamed, `callback$1`, where
it shadows the parameter of the function it's in, `callback`: Rust's
closure shadows it too, never reading the parameter, and JS's block
scope reads the same, so a rename would be one the program didn't write.

Held by `test/lowering-numbers.test.ts`, "an index an item is there at
counts on without its wrap", run in JS, `present_index_counts.rs`
against native Rust, and the mutations `present-index-unknown`,
`present-index-where-it-isnt` (`body_queries/ranges.ts`),
`present-not-through-a-closure` and `present-below-most-itself`
(`body_queries/ranges/present.ts`).

## Amendment: a cell an `Rc` holds is read through it

A `RefCell` shared by its `Rc`, `listeners: Rc<RefCell<Vec<&dyn Fn()>>>`,
is borrowed through the `Rc`'s `deref`, a call: `while i <
listeners.borrow().len() { let listener = listeners.borrow()[i];
listener(); i += 1 }` read no slice of the test, so it showed nothing of
`i`, and printed `$index(listeners, i)` and `i = (i + 1) >>> 0`.
**An `Rc`'s or an `Arc`'s `deref` of a `RefCell`, std's, which runs
nothing, is a view of the cell** (`recognition::cell_view`'s
`CellView::Cell`), as long as the `Rc` is: the same cell each time it's
read, its `Vec` the slice `([CELL], listeners)`. So `listeners[i]` and
`i++`, as upstream's. What may change the `Vec` meanwhile still forgets
it, `listener()` here, a call that may push another, which the next
test reads anew.

Case N: neither check can fail. Held by
`test/corpus/refcell_listeners_momentary.rs` against native Rust, and
the mutation `rc-deref-of-a-cell-unviewed` (`recognition/types.ts`).

## Amendment: an item written by an operation keeps its index

`pattern_table[suffix_index] = prefix_index as u32 + 1; suffix_index += 1;`
of trekhleb/javascript-algorithms' Knuth-Morris-Pratt printed `const index
= suffixIndex; patternTable[index] = ..` (#353). Its write, `(*_r) =
Add(move _24, const 1_u32)` after `index_mut`, wasn't of a value made
before, which only a local or a constant was taken to be. **An operation
of locals and constants, a binary or unary one or a cast, runs nothing
either**: MIR's operations don't panic, as each check is an `Assert`
before them. So its index is read where it's written,
`patternTable[suffixIndex] = (prefixIndex + 1) >>> 0;`, as upstream's.

Case N: nothing changes the index between its read and the write.
Held by `test/corpus/index_written_by_an_operation.rs` against native
Rust. It writes an addition, a negation and a cast, and a division by
zero panics before the bounds check, as Rust's does. Mutations
`written-at-once-of-an-operation-held`, `-a-negation-held` and
`-a-cast-held` (`mir/bounds.ts`).
