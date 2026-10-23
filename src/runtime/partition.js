// `partition(keep)` of `items`, an array or an iterator: `[yes, no]`, each
// item put in one as it comes, by std's `for_each`, which an iterator
// rust-js makes routes through the crate's own `fold` (#162, #179).
function $partition(items, keep) {
  const yes = [];
  const no = [];
  Iterator.from(items).forEach((item) => {
    (keep(item) ? yes : no).push(item);
  });
  return [yes, no];
}
