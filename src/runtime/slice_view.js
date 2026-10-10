// `&mut v[start..end]` (ADR 0335): those items of `items`, an array to JS's
// own methods, whose every read and write is one of `items`. Checked as
// `&v[start..end]` is, as it's made. Nothing a `&mut [T]` does makes it
// longer or shorter, so it stays over the same items.
function $view(items, start, end = items.length) {
  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length);
  const length = end - start;
  const at = (key) => {
    if (typeof key !== "string") return -1;
    const i = Number(key);
    return Number.isInteger(i) && i >= 0 && i < length && String(i) === key ? i : -1;
  };
  return new Proxy([], {
    get(target, key) {
      if (key === "length") return length;
      const i = at(key);
      return i < 0 ? Reflect.get(target, key) : items[start + i];
    },
    set(target, key, value) {
      // `$assign`'s, of an array as long (ADR 0147).
      if (key === "length") return value === length;
      const i = at(key);
      if (i < 0) return false;
      items[start + i] = value;
      return true;
    },
    has(target, key) {
      return at(key) >= 0 || Reflect.has(target, key);
    },
    deleteProperty() {
      return false;
    },
    ownKeys() {
      return [...Array.from({ length }, (_, i) => String(i)), "length"];
    },
    getOwnPropertyDescriptor(target, key) {
      if (key === "length") return { value: length, writable: true, enumerable: false, configurable: false };
      const i = at(key);
      return i < 0 ? undefined : { value: items[start + i], writable: true, enumerable: true, configurable: true };
    },
  });
}

// `v.get_mut(start..end)`: a view, or `undefined`, `None`, where
// `&mut v[start..end]` would panic.
function $viewGet(v, start, end = v.length) {
  return start <= end && end <= v.length ? $view(v, start, end) : undefined;
}

// `split_at_mut(mid)`: views of the items before and after `mid`; or,
// `checked`, `undefined` where `mid` is past the end.
function $splitAtMut(v, mid, checked) {
  if (mid > v.length) {
    if (checked) return undefined;
    throw new Error("mid > len");
  }
  return [$view(v, 0, mid), $view(v, mid)];
}

// `split_first_mut()`, or `split_last_mut()` (`last`): the item at that end,
// a handle on a number or text (`handle`), and a view of the rest; or
// `undefined` of an empty slice.
function $splitEndMut(v, last, handle) {
  if (v.length === 0) return undefined;
  const at = last ? v.length - 1 : 0;
  return [handle ? $mutAt(v, at) : v[at], last ? $view(v, 0, at) : $view(v, 1)];
}

function $chunksMut(v, n) {
  return $chunks(v, n, $view);
}

function $chunksExactMut(v, n) {
  return $chunksExact(v, n, $view);
}

function $rchunksMut(v, n, exact) {
  return $rchunks(v, n, exact, $view);
}

function $sliceSplitByMut(v, p, n, inclusive, back) {
  return $sliceSplitBy(v, p, n, inclusive, back, $view);
}

function $chunkByMut(v, p) {
  return $chunkBy(v, p, $view);
}

function $splitChunkMut(v, n, last) {
  return $splitChunk(v, n, last, $view);
}
