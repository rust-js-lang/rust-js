// `BinaryHeap::pop`: `None` for an empty one, else its top, sifted out;
// `Some` of it boxed, `boxed`, where it could look like `None` (ADR 0051).
// Of items with a destructor whose comparison may panic, `drop`: where the
// sift's comparison throws, the top it took dropped, as std's `pop` drops
// it as it unwinds (#901).
function $heapPop(heap, cmp, boxed, drop) {
  if (heap.length === 0) {
    return undefined;
  }
  let top = heap.pop();
  if (heap.length > 0) {
    [top, heap[0]] = [heap[0], top];
    $unwinds(
      () => $siftDownToBottom(heap, cmp),
      drop && (() => drop(top)),
    );
  }
  return boxed ? $some(top) : top;
}
