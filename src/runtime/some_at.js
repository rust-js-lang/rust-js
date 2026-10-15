
// `Some` of the item at `index`, or `None` when there's none there.
function $someAt(items, index) {
  return index >= 0 && index < items.length ? $some(items[index]) : undefined;
}

// `Some` of a map's value at `key`, boxed where it looks like `None`, or
// `None` when there's none there.
function $someGet(map, key) {
  return map.has(key) ? $some(map.get(key)) : undefined;
}
