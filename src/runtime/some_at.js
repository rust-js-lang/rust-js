
// `Some` of the item at `index`, or `None` when there's none there.
function $someAt(items, index) {
  return index >= 0 && index < items.length ? $some(items[index]) : undefined;
}

// `Some` of a map's value at `key`, boxed where it looks like `None`, or
// `None` when there's none there: of a map of its keys' own order, one
// search (#926).
function $someGet(map, key) {
  if (map.found !== undefined) {
    const found = map.found(key);
    return found.there ? $some(found.value) : undefined;
  }
  return map.has(key) ? $some(map.get(key)) : undefined;
}

// `Some` of what an optional call gave, `value.method?.(args)`, its `null`
// boxed, or `None` where it's `undefined`, as `?.` gives of a method that
// isn't there (#1859).
function $someDefined(value) {
  return value === undefined ? undefined : $some(value);
}

