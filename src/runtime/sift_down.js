// `sift_down_range(pos, end)`, std's: the item at `pos` moved down past
// each child greater than it. Where a comparison throws, a panic, it's
// written where its hole is, as std's `Hole` writes it as it unwinds, so
// the heap holds each item once (#901).
function $siftDown(heap, pos, end, cmp) {
  const item = heap[pos];
  try {
    let child = 2 * pos + 1;
    while (child <= end - 2) {
      if (cmp(heap[child], heap[child + 1]) <= 0) {
        child++;
      }
      if (cmp(item, heap[child]) >= 0) {
        return pos;
      }
      heap[pos] = heap[child];
      pos = child;
      child = 2 * pos + 1;
    }
    if (child === end - 1 && cmp(item, heap[child]) < 0) {
      heap[pos] = heap[child];
      pos = child;
    }
    return pos;
  } finally {
    heap[pos] = item;
  }
}

// `sift_down_to_bottom(0)`, `pop`'s: the item at the top moved down past
// the greater child each time to the bottom, then sifted up; its hole
// written as a comparison throws, as `$siftDown`'s is.
function $siftDownToBottom(heap, cmp) {
  const end = heap.length;
  const item = heap[0];
  let pos = 0;
  try {
    let child = 1;
    while (child <= end - 2) {
      if (cmp(heap[child], heap[child + 1]) <= 0) {
        child++;
      }
      heap[pos] = heap[child];
      pos = child;
      child = 2 * pos + 1;
    }
    if (child === end - 1) {
      heap[pos] = heap[child];
      pos = child;
    }
  } finally {
    heap[pos] = item;
  }
  $siftUp(heap, 0, pos, cmp);
}
