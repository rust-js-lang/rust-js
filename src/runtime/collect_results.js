// `collect()` into a `Result`: the first `Err`, where it stops, as Rust's
// does, or `Ok` of all the values, made `into` the collection where it's
// one but an array. Stopped, what it steps is closed, dropping what's left
// of it, then each value collected is dropped by `drop`, as std's are.
// Where stepping it throws, a panic in a closure, what it collected and
// what's left are dropped as std's unwinds, what's left first,
// `restFirst`, where it doesn't collect in place (#423).
function $collectResults(items, into, drop, restFirst = false) {
  const values = [];
  let failed;
  const it = items[Symbol.iterator]();
  try {
    for (let step = it.next(); !step.done; step = it.next()) {
      const item = step.value;
      if (item.TAG === "Err") {
        failed = item;
        break;
      }
      values.push(item._0);
    }
  } catch (error) {
    $collectUnwound(it, values, drop, restFirst);
    throw error;
  }
  if (failed === undefined) return { TAG: "Ok", _0: into ? into(values) : values };
  it.return?.();
  if (drop) $dropItems(values, drop);
  return failed;
}
