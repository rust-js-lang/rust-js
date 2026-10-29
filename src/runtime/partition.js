// `partition(keep)` of `items`, an array or an iterator: `[yes, no]`, each
// item put in one as it comes, by std's `for_each`, which an iterator
// rust-js makes routes through the crate's own `fold` (#162, #179).
//
// Of items with a destructor, `drop`: where `keep` throws, a panic, the item
// it was asked of dropped, then what's left of `items`, an array's too, as
// std's `fold` drops what it took, then `no`'s, then `yes`'s, as
// `partition`'s own unwind in reverse (#423).
function $partition(items, keep, drop) {
  const yes = [];
  const no = [];
  const it = Array.isArray(items) && drop !== undefined ? $closing($iter(items), $dropEach(drop)) : Iterator.from(items);
  try {
    it.forEach((item) => {
      let kept;
      try {
        kept = keep(item);
      } catch (error) {
        drop?.(item);
        throw error;
      }
      (kept ? yes : no).push(item);
    });
  } catch (error) {
    if (drop !== undefined) {
      try {
        $dropItems(no, drop);
      } finally {
        $dropItems(yes, drop);
      }
    }
    throw error;
  }
  return [yes, no];
}
