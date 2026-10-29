// `collect()` into an `Option`: `None` at the first `None`, where it stops,
// as Rust's does, or all the values, each out of its box where it has one
// (ADR 0051), made `into` the collection where it's one but an array.
// Stopped, what it steps is closed, dropping what's left of it, then each
// value collected is dropped by `drop`, as std's are. Where stepping it
// throws, a panic in a closure, what it collected and what's left are
// dropped as std's unwinds, what's left first, `restFirst`, where it
// doesn't collect in place (#423).
function $collectOptions(items, boxed = false, into, drop, restFirst = false) {
  const values = [];
  let failed = false;
  const it = items[Symbol.iterator]();
  try {
    for (let step = it.next(); !step.done; step = it.next()) {
      const item = step.value;
      if (item == null) {
        failed = true;
        break;
      }
      values.push(boxed ? $someValue(item) : item);
    }
  } catch (error) {
    $collectUnwound(it, values, drop, restFirst);
    throw error;
  }
  if (!failed) return into ? into(values) : values;
  it.return?.();
  if (drop) $dropItems(values, drop);
  return undefined;
}
