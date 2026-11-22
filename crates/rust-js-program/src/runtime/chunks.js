// What a slice's chunks or windows were made of, kept on the array of them
// `$chunks` and `$windows` make: its source, its size, and whether its
// parts are windows. std's `Debug` of one shows what it has left as its own
// struct holds it, `Chunks { v: [3], chunk_size: 2 }` (#2252).
const $partsOf = Symbol("rust-js.partsOf");

// `chunks(size)`: each `size` items, the last fewer, as `cut` cuts them:
// copies, or views of `chunks_mut`'s (ADR 0335).
function $chunks(v, size, cut = $part) {
  if (size === 0) {
    throw new Error("chunk size must be non-zero");
  }
  const parts = Array.from({ length: Math.ceil(v.length / size) }, (_, i) => cut(v, i * size, Math.min(i * size + size, v.length)));
  parts[$partsOf] = { source: v, size, windows: false };
  return parts;
}

// What a slice's chunks or windows, `it`, the array of them or its `$iter`
// as stepping leaves it, has left of its source, as std's struct's `v`
// holds it: of chunks `at` to `end` (and their source's end), the slice
// they cover; of windows, from the first left to the last's end, which is
// what std keeps of a slice shorter than its window, or once it's done
// (#2252). Read where it's shown: a copy, as `{:?}` reads it.
function $partsLeft(it) {
  const parts = $arrayIter(it) ? it.items : it;
  const [at, end] = $arrayIter(it) ? [it.at, it.end] : [0, parts.length];
  const { source, size, windows } = parts[$partsOf];
  if (windows) return $part(source, at, Math.min(end + size - 1, source.length));
  return $part(source, Math.min(at * size, source.length), Math.min(end * size, source.length));
}

// The size a slice's chunks or windows, `it`, were made of: std's
// `chunk_size` or `size` (#2252).
function $partsSize(it) {
  return ($arrayIter(it) ? it.items : it)[$partsOf].size;
}

// What a slice's chunks or windows, `it`, a `$iter` of them, has left, as
// an array of them that keeps what they were cut of, of that part of its
// source, so what's made of it, a clone, shows what std's does (#2252):
// of chunks from the first left's start, of windows from the first left.
function $partsStepped(it) {
  const parts = it.items.slice(it.at, it.end);
  const { source, size, windows } = it.items[$partsOf];
  const from = windows ? Math.min(it.at, source.length) : Math.min(it.at * size, source.length);
  parts[$partsOf] = { source: $part(source, from, source.length), size, windows };
  return parts;
}

// What an iterator of one item at most, `it`, an `Option`'s or `once(x)`'s
// `$iter`, has left, as std's `Item`'s `opt` holds it: its item, boxed
// where its type looks like `None` (ADR 0051), else nothing (#2252).
function $itemLeft(it, boxed) {
  if (it.at >= it.end) return undefined;
  return boxed ? $some(it.items[it.at]) : it.items[it.at];
}
