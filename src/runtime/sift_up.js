// `sift_up(start, pos)`, std's: the item at `pos` moved up past each parent
// less than it. Where a comparison throws, a panic, it's written where its
// hole is, as std's `Hole` writes it as it unwinds (#901).
function $siftUp(heap, start, pos, cmp) {
  const item = heap[pos];
  try {
    while (pos > start) {
      const parent = (pos - 1) >>> 1;
      if (cmp(item, heap[parent]) <= 0) {
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
