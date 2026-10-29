
// `collect()` into an `Option`: `None` at the first `None`, where it stops,
// as Rust's does, or all the values, each out of its box where it has one
// (ADR 0051), made `into` the collection where it's one but an array.
// Stopped, what it steps is closed, dropping what's left of it, then each
// value collected is dropped by `drop`, as std's are.
function $collectOptions(items, boxed = false, into, drop) {
  const values = [];
  let failed = false;
  for (const item of items) {
    if (item == null) {
      failed = true;
      break;
    }
    values.push(boxed ? $someValue(item) : item);
  }
  if (!failed) return into ? into(values) : values;
  if (drop) $dropItems(values, drop);
  return undefined;
}
