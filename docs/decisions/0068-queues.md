# 0068. `VecDeque` and `BinaryHeap` are arrays; a heap moves its items as Rust's does

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md), [0057](0057-ordering.md) and [0064](0064-numbers.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Graph searches use a queue (`VecDeque`) and a priority queue (`BinaryHeap`,
often of `Reverse(..)` for a min-heap). Both were errors. So was a crate's
own `Index` impl, as a grid's `g[(r, c)]` is.

And `Reverse` wasn't an error, but was wrong: it was compared field by
field, like a struct whose `Ord` is derived, so `Reverse(1) < Reverse(2)`
was `true`, and `v.sort_by_key(|&x| Reverse(x))` sorted up, not down.

## Decision

**A `VecDeque` is a JS array**, as a `Vec` is (ADR 0036), with its own
methods:

| Rust | JS |
|---|---|
| `d.push_back(x)`, `d.pop_back()` | `d.push(x)`, `d.pop()` |
| `d.push_front(x)`, `d.pop_front()` | `d.unshift(x)`, `d.shift()` |
| `d.front()`, `d.back()` | `d[0]`, `d.at(-1)` |
| `d.remove(i)` | `$removeOpt(d, i)`: an `Option`, where `Vec`'s panics |
| `VecDeque::from(v)` | `v.slice()` |

**A `BinaryHeap` is a JS array in the order Rust's heap keeps it**, moved by
the same steps: `$heapPush` is Rust's `sift_up`, `$heapPop` its
`sift_down_to_bottom`, `$heapSorted` its `into_sorted_vec`, and `$heapFrom`
its `rebuild`. Each takes the items' `cmp` (ADR 0057):

```js
$heapPush(heap, [[0, src]], (a, b) => $cmp(b[0][0], a[0][0]) || $cmp(b[0][1], a[0][1]));
```

- **It's Rust's order, not just a heap's.** Which of two equal items comes
  out first, and what `{:?}`, `iter()` and `into_vec()` show, depend on how
  the items are moved. The same steps give the same answers.
- `peek()` is `heap[0]`, and `collect()` into a heap, like `from`, puts the
  items in heap order.
- **What Rust consumes is copied:** `into_sorted_vec`, `BinaryHeap::from`
  and `VecDeque::from` take a value that may be a clone never made (ADR
  0052), which changing in place would change for its original too.
- **A heap of items that could look like `None`** is an error, since `pop`
  is `undefined` for `None`.

**`Reverse` compares the other way round**, as its impl does:
`cmp(Reverse(a), Reverse(b))` is `cmp(b, a)`. `==`, `{:?}` and `clone` are
its field's, as derived.

**Another crate's struct is no longer compared field by field**, which is
right only for a derived `Ord`. It's an error, unless rust-js knows how it
orders (`Option`, `Result`, `Reverse`).

### Also here

- `impl Index<(usize, usize)> for Grid`: `g[(1, 2)]` calls the impl, as an
  operator does (ADR 0064). `IndexMut` is still an error.
- A constant index into a variable (`t[0]`) is read in place, like `t.x`,
  rather than put in a `const` first: comparators are one line.
- `[3, 4].slice()` is `[3, 4]`: an array just written needs no copy.

## Why

- **It's Rust's answer.** Differential tests push nine tasks whose `Ord`
  compares only their priorities, and check the heap's order after each
  push and pop, `into_sorted_vec`, `from` and `collect`.
- **It's the JS a person writes** for a queue, and a heap in a few small
  functions.

## Alternatives

- **Any binary heap**, or sorting on each `pop`: simpler, but equal items
  would come out in another order, and `{:?}` of a heap would differ.
- **A ring buffer for `VecDeque`**, for an O(1) `pop_front`. `shift` is
  O(n), but it's what JS code uses, and it keeps the deque an array that
  JS can read.

## Consequences

- A long queue drained with `pop_front` costs O(n²) in JS.

## Amendment: a heap of `()` is refused where it's popped

A heap whose items could look like `None`, `()`'s, was an error wherever it
was used, its own check, `heap_of`. Only its `pop` is an `Option` of an
item, `undefined` for `None`, which is refused as any such `Option` is
(ADR 0051), the one check that answers it: `heap_of` is gone. `peek` boxes
a `Some` that would look like it, as a slice's `first` does, so a heap of
`()` is pushed, counted and peeked as any is. Its items, all `Equal`,
compare as `()` does (ADR 0057).

## Amendment: a `Vec` of a slice, of a deque and of text

`Vec::from(&v[..])`, a deque's `into()` a `Vec`, and `Vec::<u8>::from(s)`
of text were refused (#116). A `Vec` of a slice is its items cloned, of a
deque its items, as `VecDeque::from(v)` is, `Std::ToVec`; of text its
bytes, as `into_bytes()` gives them. Held by
`test/corpus/vec_from_items.rs`.

## Amendment: a heap's `pop()` of what could look like `None` (#318)

`pop()` of a `BinaryHeap` of a type parameter's items, which could be
`Option`s, was refused, "this call, for an `Option` of what could look like
`None`": TheAlgorithms/rust's Prüfer code, `queue.pop().unwrap()` of a
`BinaryHeap<V>`. **Its `Some` is boxed as a `Vec`'s `pop()`'s is** (ADR
0051): `$heapPop(heap, VOrd.cmp, true)`, given `boxed`, gives `$some(top)`,
so a `None` the heap holds is `Some(None)`, not an empty heap's `None`; of
other items, as before. Held by the corpus case `heap_pop_of_a_param` and
the mutations `heap-pop-unboxed` and `heap-pop-box-ignored`.


## Amendment: a sift whose comparison panics keeps every item once (#901)

std's `sift_up`, `sift_down_range` and `sift_down_to_bottom` hold the item
they move in a `Hole`, which writes it where the hole is as a comparison
unwinds. The runtime's sifts moved the item over its hole with nothing to
write it back: where a comparison of the crate's `Ord` panicked, the
hole's slot kept the item moved over it, so the heap dropped that one twice
and the moving one never. Results change (ADR 0262): Rust's behaviour is
kept.

- **Each sift writes its item where its hole is as it unwinds**, `$siftUp`,
  `$siftDown` and `$siftDownToBottom`, `pop`'s: a `finally`, as std's
  `Hole`'s drop. It costs nothing to read: the runtime's own.
- **What std's frame holds is dropped as it unwinds**, of items with a
  destructor whose comparison may panic (`order_may_panic`, ordering.rs,
  the one answer, of a hand-written or derived order's impls, ADR 0902,
  and a derived one's fields): `pop`'s top, `$heapPop(h, cmp, boxed,
  drop)`; `into_sorted_vec`'s and `from`'s items, the heap they were
  making, `$heapSorted(h, cmp, drop)` and `$heapFrom(v, cmp, drop)`. Of
  an order that never panics, the call it always was.

The corpus's `heap_sift_unwinds` runs `push`, `pop`, `append`, `extend`,
`into_sorted_vec` and `from`, of a type with its own `Ord` and of a struct
whose derived order is its, under each panic point, against native Rust;
`test/lowering-std-unwinds.test.ts` pins both forms; the unwind table's 84
case-builds of #901 pass.
