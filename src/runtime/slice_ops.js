// A slice's fills, copies, order checks, chunks and splits (ADR 0324).

// `v.fill(x)`: a clone of `x` in each but the last, which is `x` itself, as
// std's is; `clone` where a clone is more than the value.
function $fill(v, x, clone) {
  if (!clone) return v.fill(x);
  for (let i = 0; i < v.length - 1; i++) v[i] = clone(x);
  if (v.length > 0) v[v.length - 1] = x;
}

function $fillWith(v, f) {
  for (let i = 0; i < v.length; i++) v[i] = f();
}

function $copyFromSlice(v, src) {
  if (v.length !== src.length) {
    throw new Error(`copy_from_slice: source slice length (${src.length}) does not match destination slice length (${v.length})`);
  }
  for (let i = 0; i < v.length; i++) v[i] = src[i];
}

// `clone_from_slice(src)`: each item a clone, or `cloneFrom(v, i, item)`'s,
// its type's own `clone_from` (ADR 0324).
function $cloneFromSlice(v, src, clone, cloneFrom) {
  if (v.length !== src.length) throw new Error("destination and source slices have different lengths");
  for (let i = 0; i < v.length; i++) {
    if (cloneFrom) cloneFrom(v, i, src[i]);
    else v[i] = clone ? clone(src[i]) : src[i];
  }
}

function $swapWithSlice(v, other) {
  if (v.length !== other.length) throw new Error("destination and source slices have different lengths");
  for (let i = 0; i < v.length; i++) [v[i], other[i]] = [other[i], v[i]];
}

function $copyWithin(v, start, end, dest) {
  end ??= v.length;
  $checkRange(start, end, v.length);
  if (dest > v.length - (end - start)) throw new Error("dest is out of bounds");
  v.copyWithin(dest, start, end);
}

// `is_sorted_by(f)`: `f` of each two in a row, `true` for all.
function $isSortedBy(v, f) {
  for (let i = 1; i < v.length; i++) if (!f(v[i - 1], v[i])) return false;
  return true;
}

// `is_sorted_by_key(f, cmp)`: `f` of each item once, as an iterator's is, up
// to the first out of order.
function $isSortedByKey(v, f, cmp) {
  if (v.length === 0) return true;
  let last = f(v[0]);
  for (let i = 1; i < v.length; i++) {
    const key = f(v[i]);
    if (!(cmp(last, key) <= 0)) return false;
    last = key;
  }
  return true;
}

// `partition_point(p)`: where `p` stops holding, by std's binary search.
function $partitionPoint(v, p) {
  let size = v.length;
  if (size === 0) return 0;
  let base = 0;
  while (size > 1) {
    const half = size >> 1;
    const mid = base + half;
    if (p(v[mid])) base = mid;
    size -= half;
  }
  return base + (p(v[base]) ? 1 : 0);
}

function $chunkSize(n) {
  if (n === 0) throw new Error("chunk size must be non-zero");
}

// What a slice's chunks and splits are cut as: copies, or views of the
// `_mut` ones' (ADR 0335).
function $copyOf(items, start, end) {
  return items.slice(start, end);
}

// `chunks_exact(n)`: each whole chunk, and what's left, its `remainder()`.
function $chunksExact(v, n, cut = $copyOf) {
  $chunkSize(n);
  const whole = v.length - (v.length % n);
  const chunks = [];
  for (let i = 0; i < whole; i += n) chunks.push(cut(v, i, i + n));
  chunks.remainder = cut(v, whole, v.length);
  return chunks;
}

// `rchunks(n)`, and `rchunks_exact(n)`: chunks from the end; the exact ones
// leave what's left at the start, their `remainder()`.
function $rchunks(v, n, exact, cut = $copyOf) {
  $chunkSize(n);
  const chunks = [];
  let end = v.length;
  for (; end >= n; end -= n) chunks.push(cut(v, end - n, end));
  if (exact) chunks.remainder = cut(v, 0, end);
  else if (end > 0) chunks.push(cut(v, 0, end));
  return chunks;
}

// `split(p)` of a slice: the pieces between the items `p` holds of, at most
// `n`, each with its item where `inclusive`, and from the end where `back`.
function $sliceSplitBy(v, p, n = Infinity, inclusive = false, back = false, cut = $copyOf) {
  if (n === 0) return [];
  const parts = [];
  if (back) {
    let end = v.length;
    for (let i = v.length - 1; i >= 0 && parts.length < n - 1; i--) {
      if (p(v[i])) {
        parts.push(cut(v, i + 1, end));
        end = i;
      }
    }
    parts.push(cut(v, 0, end));
    return parts;
  }
  let start = 0;
  for (let i = 0; i < v.length && parts.length < n - 1; i++) {
    if (p(v[i])) {
      parts.push(cut(v, start, inclusive ? i + 1 : i));
      start = i + 1;
    }
  }
  if (!inclusive || start < v.length) parts.push(cut(v, start, v.length));
  return parts;
}

// `as_chunks::<N>()`: its whole chunks of `n`, copies, and what's left after
// them; `as_rchunks` (`back`): what's left before them first.
function $asChunks(v, n, back = false) {
  $chunkSize(n);
  const rest = v.length % n;
  const start = back ? rest : 0;
  const chunks = Array.from({ length: (v.length - rest) / n }, (_, i) => v.slice(start + i * n, start + i * n + n));
  return back ? [v.slice(0, rest), chunks] : [chunks, v.slice(v.length - rest)];
}

// `chunk_by(p)`: the runs of items `p` holds of each two in a row of.
function $chunkBy(v, p, cut = $copyOf) {
  const runs = [];
  let start = 0;
  for (let i = 1; i <= v.length; i++) {
    if (i === v.length || !p(v[i - 1], v[i])) {
      runs.push(cut(v, start, i));
      start = i;
    }
  }
  return runs;
}

// `split_first_chunk::<N>()`, or `split_last_chunk` (`last`): its first
// `n`, or its last, and the rest; `undefined` where it has fewer.
function $splitChunk(v, n, last, cut = $copyOf) {
  if (v.length < n) return undefined;
  const at = last ? v.length - n : n;
  return [cut(v, 0, at), cut(v, at, v.length)];
}

// `sort_by_cached_key(f)`: `f` of each item once, in order, then sorted by
// those keys, ties in the order they were.
function $sortByCachedKey(v, f, cmp) {
  const keyed = v.map((item, i) => [f(item), i, item]);
  keyed.sort((a, b) => cmp(a[0], b[0]) || a[1] - b[1]);
  keyed.forEach(([, , item], i) => (v[i] = item));
}

// A byte slice's ASCII case: a copy, `to_ascii_uppercase()`, or in place,
// `make_ascii_uppercase()`.
function $asciiBytes(v, upper) {
  return v.map((b) => (upper ? (b >= 97 && b <= 122 ? b - 32 : b) : b >= 65 && b <= 90 ? b + 32 : b));
}

function $makeAsciiBytes(v, upper) {
  $asciiBytes(v, upper).forEach((b, i) => (v[i] = b));
}

// A byte slice's `trim_ascii()`: without the ASCII whitespace at its ends.
function $trimAsciiBytes(v, start, end) {
  const space = (b) => b === 32 || b === 9 || b === 10 || b === 12 || b === 13;
  let from = 0;
  let to = v.length;
  while (start && from < to && space(v[from])) from++;
  while (end && to > from && space(v[to - 1])) to--;
  return v.slice(from, to);
}

// `v.strip_prefix(p)`, or `strip_suffix` (`end`): the rest, or `None`, of
// items `==` compares by value.
function $sliceStrip(v, p, end, eq) {
  if (!$sliceStartsWith(v, p, end, eq)) return undefined;
  return end ? v.slice(0, v.length - p.length) : v.slice(p.length);
}

// `strip_circumfix(prefix, suffix)`: `strip_prefix`'s, then `strip_suffix`'s
// of what's left.
function $sliceStripCircumfix(v, prefix, suffix, eq) {
  const rest = $sliceStrip(v, prefix, false, eq);
  return rest === undefined ? undefined : $sliceStrip(rest, suffix, true, eq);
}

// `v.repeat(n)`: its items `n` times over, of `Copy` items.
function $repeatItems(v, n) {
  const out = [];
  for (let i = 0; i < n; i++) out.push(...v);
  return out;
}

// A `VecDeque`'s `swap_remove_back(i)`, or `swap_remove_front(i)`: the item
// at `i`, its last item, or its first, in its place; `None` past the end.
function $dequeSwapRemove(v, i, front) {
  if (i >= v.length) return undefined;
  const item = v[i];
  if (front) {
    v[i] = v[0];
    v.shift();
  } else {
    v[i] = v[v.length - 1];
    v.pop();
  }
  return item;
}

// `retain_mut(f)`: the items `f` keeps, each given as itself, or as a handle
// on a number or a string, whose change is kept.
function $retainMut(v, f, handles) {
  let kept = 0;
  for (let i = 0; i < v.length; i++) {
    const at = i;
    const keep = handles
      ? f({
          get value() {
            return v[at];
          },
          set value(item) {
            v[at] = item;
          },
        })
      : f(v[at]);
    if (keep) v[kept++] = v[at];
  }
  v.length = kept;
}

// A `VecDeque`'s `pop_front_if(f)`: its first item, if `f` of it holds.
function $popFrontIf(v, holds, handles) {
  if (v.length === 0) return undefined;
  const given = handles
    ? {
        get value() {
          return v[0];
        },
        set value(item) {
          v[0] = item;
        },
      }
    : v[0];
  return holds(given) ? v.shift() : undefined;
}

// `push_mut(x)` and `insert_mut(i, x)`: `x` put in, and a `&mut` to it there,
// a handle on a number or text (ADR 0152).
function $pushMut(v, x, handle) {
  v.push(x);
  return handle ? $mutAt(v, v.length - 1) : x;
}

function $insertMut(v, i, x, handle) {
  $insertAt(v, i, x);
  return handle ? $mutAt(v, i) : x;
}
