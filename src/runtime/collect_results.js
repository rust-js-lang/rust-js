// `collect()` into a `Result`: the first `Err`, where it stops, as Rust's
// does, or `Ok` of all the values, made `into` the collection where it's
// one but an array. Stopped, what it steps is closed, dropping what's left
// of it, then each value collected is dropped by `drop`, as std's are.
// Where stepping it throws, a panic in a closure, what it collected and
// what's left are dropped as std's unwinds, what's left first,
// `restFirst`, where it doesn't collect in place (#423). Of a `Vec` of
// values of `size` bytes, where its hint may run the crate's own, that
// hint asked as std's `Vec` grows of the `GenericShunt` (`$shuntGrowth`):
// where asking it panics, the value just taken dropped first (#848).
function $collectResults(items, into, drop, restFirst = false, size) {
  const values = [];
  let failed;
  const it = items[Symbol.iterator]();
  const grown = size === undefined ? undefined : $shuntGrowth(it, size);
  $collectUnwinding(() => {
    // Each step `GenericShunt`'s, by its source's `try_fold` (#1004).
    for (let step = $shuntStep(it); !step.done; step = $shuntStep(it)) {
      const item = step.value;
      if (item.TAG === "Err") {
        failed = item;
        break;
      }
      $unwinds(
        () => grown?.(values.length),
        () => drop?.(item._0),
      );
      values.push(item._0);
    }
  }, it, values, drop, restFirst);
  if (failed === undefined) return { TAG: "Ok", _0: into ? into(values) : values };
  it.return?.();
  if (drop) $dropItems(values, drop);
  return failed;
}
