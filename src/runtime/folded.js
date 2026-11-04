// The items of `it` its `fold` gives, the crate's own its impl writes,
// listed as they're given and stepped after: what a consumer std routes
// through `fold` takes of such an iterator, `last` and `collect` into a
// string, a hash map or set, a list (#162). Of a JS iterator with no
// `fold`, JS's own of an array in generic code, itself, whose `next` is all
// std's fold would step (#182). Listed when first stepped; a consumer that
// folds it, `$lastDropping`, folds `it` itself, so what that consumer drops
// as each comes is dropped before what `it` drops at its end, as std's
// fold of it is (#493). `firstByNext`, of a consumer that takes its first
// item by `next` and folds the rest, a `String` collected of `String`s,
// whose first step is a `skip`'s `nth(n)`, not its fold's `nth(n - 1)`
// (#494).
function $folded(it, firstByNext = false) {
  if (it.fold === undefined) return it;
  const foldOf = (init, f, unwind) => {
    if (!firstByNext) return it.fold(init, f, unwind);
    const first = it.next();
    return first.done ? init : it.fold(f(init, first.value), f, unwind);
  };
  let items;
  const listed = () =>
    (items ??= foldOf([], (list, item) => {
      list.push(item);
      return list;
    }).values());
  const folded = Iterator.from({ next: () => listed().next() });
  return $forward(folded, it, {
    fold: (init, f, unwind) => (items === undefined ? foldOf(init, f, unwind) : listed().reduce(f, init)),
  });
}
