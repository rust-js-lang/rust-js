// `&mut v[start..end]` (ADR 0335): those items of `items`, an array to JS's
// own methods, whose every read and write is one of `items`. Checked as
// `&v[start..end]` is, as it's made. Nothing a `&mut [T]` does makes it
// longer or shorter, so it stays over the same items.
function $view(items, start, end = items.length) {
  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length);
  return $proxy(
    end - start,
    (i) => items[start + i],
    (i, item) => {
      items[start + i] = item;
    },
  );
}

// An array of `length` items to JS's own methods, each read by `read(i)`
// and written by `write(i, item)`.
function $proxy(length, read, write) {
  const at = (key) => {
    if (typeof key !== "string") return -1;
    const i = Number(key);
    return Number.isInteger(i) && i >= 0 && i < length && String(i) === key ? i : -1;
  };
  return new Proxy([], {
    get(target, key) {
      if (key === "length") return length;
      const i = at(key);
      return i < 0 ? Reflect.get(target, key) : read(i);
    },
    set(target, key, value) {
      // `$assign`'s, of an array as long (ADR 0147).
      if (key === "length") return value === length;
      const i = at(key);
      if (i < 0) return false;
      write(i, value);
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
      return i < 0 ? undefined : { value: read(i), writable: true, enumerable: true, configurable: true };
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

// `as_chunks_mut::<N>()`: a view of its whole chunks of `n`, each a view,
// written whole as `chunks[i] = [..]` writes one, and a view of what's left
// after them; `as_rchunks_mut` (`back`): what's left before them first.
// A chunk read before its place is written keeps what it held, as Rust's
// copy of a `[T; N]` does: JS's own `reverse` and `sort`, and `$swap`,
// hold one while they write its place.
function $asChunksMut(v, n, back = false) {
  $chunkSize(n);
  const rest = v.length % n;
  const start = back ? rest : 0;
  const read = [];
  const chunks = $proxy(
    (v.length - rest) / n,
    (i) => {
      const at = start + i * n;
      let kept;
      (read[i] ??= []).push(() => {
        kept = v.slice(at, at + n);
      });
      return $proxy(
        n,
        (j) => (kept ? kept[j] : v[at + j]),
        (j, item) => {
          if (kept) kept[j] = item;
          else v[at + j] = item;
        },
      );
    },
    (i, chunk) => {
      const items = Array.from(chunk);
      for (const keep of read[i] ?? []) keep();
      read[i] = [];
      for (let j = 0; j < n; j++) v[start + i * n + j] = items[j];
    },
  );
  return back ? [$view(v, 0, rest), chunks] : [chunks, $view(v, v.length - rest)];
}

// `as_flattened_mut()` of arrays of `n`: a view of their items, in order.
function $flatView(v, n) {
  return $proxy(
    v.length * n,
    (i) => v[Math.floor(i / n)][i % n],
    (i, item) => {
      v[Math.floor(i / n)][i % n] = item;
    },
  );
}

function $sliceSplitOffMut(v, n, back) {
  return $sliceSplitOff(v, n, back, $view);
}

// Of a `&mut` slice, its item a handle on a number or text (`handle`).
function $sliceSplitOffEndMut(v, last, handle) {
  return $sliceSplitOffEnd(v, last, $view, handle ? $mutAt : undefined);
}

// `get_disjoint_mut(indices)` (ADR 0339): `Ok` of each index's item, a
// handle on a number or text (`handle`), or each range's view, `a..=b`
// where `inclusive`; or `Err` of why not, checked as std checks them, in
// order: each in bounds, then apart from those before it.
function $getDisjointMut(v, indices, handle = false, inclusive = false) {
  const bounds = indices.map((at) => (typeof at === "number" ? [at, at + 1] : [at.start, inclusive ? at.end + 1 : at.end]));
  for (let i = 0; i < bounds.length; i++) {
    const [start, end] = bounds[i];
    // `a..=b` is in bounds where `a <= b`: where `a` is before `b + 1`.
    if (start > end || (inclusive && start === end) || end > v.length) return { TAG: "Err", _0: "IndexOutOfBounds" };
    for (let j = 0; j < i; j++) {
      if (start < bounds[j][1] && bounds[j][0] < end) return { TAG: "Err", _0: "OverlappingIndices" };
    }
  }
  const take = (at, i) => (typeof at !== "number" ? $view(v, ...bounds[i]) : handle ? $mutAt(v, at) : v[at]);
  return { TAG: "Ok", _0: indices.map(take) };
}
