// The items of `it` its `fold` gives, the crate's own its impl writes,
// listed as they're given and stepped after: what a consumer std routes
// through `fold` takes of such an iterator, `last` and `collect` into a
// string, a hash map or set, a list (#162).
function $folded(it) {
  return it
    .fold([], (items, item) => {
      items.push(item);
      return items;
    })
    .values();
}
