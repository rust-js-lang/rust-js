// `sift_up(start, pos)`, std's: the item at `pos` moved up past each parent
// less than it, by `ord`'s `<=`, its `PartialOrd`'s, as std asks (#937). Where a comparison throws, a panic, it's written where its
// hole is, as std's `Hole` writes it as it unwinds (#901).
function $siftUp(heap, start, pos, ord) {
  const item = heap[pos];
  try {
    while (pos > start) {
      const parent = (pos - 1) >>> 1;
      if (ord.le(item, heap[parent])) {
        break;
      }
      heap[pos] = heap[parent];
      pos = parent;
    }
    return pos;
  } finally {
    heap[pos] = item;
  }
}
