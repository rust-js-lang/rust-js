// A `BinaryHeap`'s `append` and `retain` (ADR 0333), by std's own steps,
// so its items are in the order std's heap keeps them.

// Make a heap of the items from `start` on, as std does: all of it again,
// or each sifted up, whichever std counts cheaper.
function $heapRebuildTail(heap, start, cmp) {
  const len = heap.length;
  if (start === len) return;
  const tail = len - start;
  const better =
    start < tail ? true : len <= 2048 ? 2 * len < tail * (31 - Math.clz32(start)) : 2 * len < tail * 11;
  if (better) {
    for (let n = len >>> 1; n > 0; ) $siftDown(heap, --n, len, cmp);
  } else {
    for (let i = start; i < len; i++) $siftUp(heap, 0, i, cmp);
  }
}

// `heap.append(&mut other)`: the smaller's items after the larger's, `other`
// left empty.
function $heapAppend(heap, other, cmp) {
  const [first, rest] = heap.length < other.length ? [other.slice(), heap.slice()] : [heap.slice(), other.slice()];
  heap.length = 0;
  heap.push(...first, ...rest);
  other.length = 0;
  $heapRebuildTail(heap, first.length, cmp);
}

// `heap.retain(f)`: what `f` keeps, in order, rebuilt from the first it
// didn't.
function $heapRetain(heap, f, cmp) {
  let from = heap.length;
  let kept = 0;
  for (let i = 0; i < heap.length; i++) {
    if (f(heap[i])) heap[kept++] = heap[i];
    else if (i < from) from = i;
  }
  heap.length = kept;
  $heapRebuildTail(heap, Math.min(from, kept), cmp);
}

// `peek_mut()`: a guard of its top, or `undefined` of an empty heap. A
// `&mut` to the top marks it changed, and a guard of a changed top sifts it
// down as it drops, as std's does; one only read compares nothing.
function $peekMut(heap, cmp) {
  return heap.length === 0 ? undefined : { heap, cmp, changed: false };
}

// The guard's `&mut` to its top: a handle on a number or text (ADR 0152).
function $peekMutTop(guard, handle) {
  guard.changed = true;
  const heap = guard.heap;
  if (!handle) return heap[0];
  return {
    get value() {
      return heap[0];
    },
    set value(value) {
      heap[0] = value;
    },
  };
}

function $peekMutDrop(guard) {
  if (guard.changed) $siftDown(guard.heap, 0, guard.heap.length, guard.cmp);
}

// `PeekMut::pop(guard)`: the top, changed or not, taken.
function $peekMutPop(guard) {
  return $heapPop(guard.heap, guard.cmp);
}
