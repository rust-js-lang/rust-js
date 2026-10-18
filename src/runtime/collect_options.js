
// `collect()` into an `Option`: `None` at the first `None`, where it stops,
// as Rust's does, or all the values, each out of its box where it has one
// (ADR 0051), made `into` the collection where it's one but an array.
function $collectOptions(items, boxed = false, into) {
  const values = [];
  for (const item of items) {
    if (item == null) return undefined;
    values.push(boxed ? $someValue(item) : item);
  }
  return into ? into(values) : values;
}
