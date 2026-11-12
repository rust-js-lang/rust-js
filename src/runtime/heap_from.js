// `BinaryHeap::from(v)`: `v` rebuilt, each parent from the last sifted
// down, as std's `rebuild` does. Of items with a destructor whose
// comparison may panic, `drop`: where a comparison throws, each item the
// heap holds dropped, in order, as std's drops the heap it was making as it
// unwinds (#901). Its capacity `v`'s, whose buffer std's heap keeps, a
// collect's grown as a `Vec`'s, not its length (#1531).
function $heapFrom(items, ord, drop) {
  const heap = items.slice();
  let n = heap.length >>> 1;
  $unwinds(
    () => {
      while (n > 0) {
        n--;
        $siftDown(heap, n, heap.length, ord);
      }
    },
    drop && (() => $dropItems(heap, drop)),
  );
  return $inPlaceCapacity(heap, items, 1, 1);
}
