# 0333. A heap's `append`, `retain`, `drain` and `peek_mut`, and a `Vec`'s `push_mut`

Status: Accepted. Extends [0068](0068-queues.md) and
[0152](0152-std-item-handles.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `BinaryHeap` is a JS array in the order std's heap keeps it (ADR 0068),
so `{:?}`, `into_vec()` and `as_slice()` show what Rust's do. Its `append`,
`retain`, `drain`, `as_slice` and `peek_mut` were errors, and so were a
`Vec`'s `push_mut` and `insert_mut`: `BinaryHeap` was 15 of 23.

## Decision

**Each is std's, by std's own steps.**

| Rust | JS |
|---|---|
| `heap.append(&mut other)` | `$heapAppend(heap, other, cmp)`: the smaller's items after the larger's, `other` emptied |
| `heap.retain(f)` | `$heapRetain(heap, f, cmp)`: what `f` keeps, in order |
| `drain()`, `as_slice()` | `heap.splice(0)`, the array |
| `peek_mut()` | `$peekMut(heap, cmp)`: a guard, `{ heap, cmp, changed }`, or `undefined` |
| `*top`, `*top = v`, `PeekMut::pop(top)` | `top.heap[0]`, `$peekMutTop(top)` writes, `$peekMutPop(top)` |
| `v.push_mut(x)`, `v.insert_mut(i, x)` | `$pushMut(v, x)`, `$insertMut(v, i, x)`: the item put in, or a handle on a number or text |

- **What's appended or kept is rebuilt as std's `rebuild_tail` rebuilds
  it**, from the first item it changed: the whole heap again, or each item
  sifted up, by std's count of which is cheaper. Each orders the array
  differently, and `as_slice()` shows it.
- **A guard's top is sifted down as it drops only if it was written**,
  through its `&mut`: one only read compares nothing, which a crate's `Ord`
  that prints shows. The guard is a value with a destructor (ADR 0098).
- **Still refused**: `capacity`, `try_reserve` and `try_reserve_exact`,
  which ask of memory JS doesn't keep.

## Why

- **It's the same program**: each heap's array is std's, item for item.
- **It's tested**: the `heap_methods` corpus case appends a larger heap to
  a smaller, a smaller to a larger, each by std's two ways of rebuilding,
  retains from the middle and from the end, and drains; `heap_peek_mut`
  changes a top, reads one, pops one, and counts a printing `Ord`'s
  comparisons; `vec_push_mut` writes numbers, objects and text through the
  `&mut`s, against native Rust. Mutations rebuild always or never, keep
  the larger heap second, rebuild a retain whole, sift a read top, sift no
  top, copy the top, leave a drained heap full, drop a guard as nothing,
  and give `push_mut`'s number, or `insert_mut`'s last item.
- `docs/std-coverage.txt`: `BinaryHeap` 20 of 23, `Vec` 36 of 48.

## Since

- **A deque's and a list's `push_back_mut`, `push_front_mut`, and a
  `VecDeque`'s `insert_mut`** (2026-10-10) are `Vec`'s, `$pushMut`,
  `$pushFrontMut` and `$dequeInsert`: a `&mut` to what they put in, a
  handle on a number or text. A `VecDeque`'s `insert` past its end panics
  with its own `index out of bounds`, where it said `Vec`'s message. The
  corpus's `deque_push_mut` and `deque_insert_past_len` run against native
  Rust.
