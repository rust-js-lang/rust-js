// `heap.into_sorted_vec()`: the top swapped to the end, the rest sifted, as
// std does. Of items with a destructor whose comparison may panic, `drop`:
// where a comparison throws, each item the heap holds dropped, in order, as
// std's `into_sorted_vec` drops its `self` as it unwinds (#901). Its
// capacity the heap's, whose buffer std sorts in place (#1531).
function $heapSorted(items, ord, drop) {
  const heap = items.slice();
  let end = heap.length;
  $unwinds(
    () => {
      while (end > 1) {
        end--;
        [heap[0], heap[end]] = [heap[end], heap[0]];
        $siftDown(heap, 0, end, ord);
      }
    },
    drop && (() => $dropItems(heap, drop)),
  );
  return $inPlaceCapacity(heap, items, 1, 1);
}
