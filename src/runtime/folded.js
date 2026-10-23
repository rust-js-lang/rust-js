// The items of `it` its `fold` gives, the crate's own its impl writes,
// listed as they're given and stepped after: what a consumer std routes
// through `fold` takes of such an iterator, `last` and `collect` into a
// string, a hash map or set, a list (#162). Of a JS iterator with no
// `fold`, JS's own of an array in generic code, itself, whose `next` is all
// std's fold would step (#182).
function $folded(it) {
  if (it.fold === undefined) return it;
  return it
    .fold([], (items, item) => {
      items.push(item);
      return items;
    })
    .values();
}
