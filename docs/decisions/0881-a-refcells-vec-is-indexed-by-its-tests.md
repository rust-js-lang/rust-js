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
