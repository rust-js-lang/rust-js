
// `collect()` into a `Result`: the first `Err`, where it stops, as Rust's
// does, or `Ok` of all the values, made `into` the collection where it's
// one but an array. Stopped, what it steps is closed, dropping what's left
// of it, then each value collected is dropped by `drop`, as std's are.
function $collectResults(items, into, drop) {
  const values = [];
  let failed;
  for (const item of items) {
    if (item.TAG === "Err") {
      failed = item;
      break;
    }
    values.push(item._0);
  }
  if (failed === undefined) return { TAG: "Ok", _0: into ? into(values) : values };
  if (drop) $dropItems(values, drop);
  return failed;
}
