
function $heapFrom(items, cmp) {
  const heap = items.slice();
  let n = heap.length >>> 1;
  while (n > 0) {
    n--;
    $siftDown(heap, n, heap.length, cmp);
  }
  return heap;
}
