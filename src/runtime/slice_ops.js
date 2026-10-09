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

function $cloneFromSlice(v, src, clone) {
  if (v.length !== src.length) throw new Error("destination and source slices have different lengths");
  for (let i = 0; i < v.length; i++) v[i] = clone ? clone(src[i]) : src[i];
}

function $swapWithSlice(v, other) {
  if (v.length !== other.length) throw new Error("destination and source slices have different lengths");
  for (let i = 0; i < v.length; i++) [v[i], other[i]] = [other[i], v[i]];
}

function $copyWithin(v, start, end, dest) {
  end ??= v.length;
  if (start > end) throw new Error(`slice index starts at ${start} but ends at ${end}`);
  if (end > v.length) throw new Error(`range end index ${end} out of range for slice of length ${v.length}`);
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

// `chunks_exact(n)`: each whole chunk, and what's left, its `remainder()`.
function $chunksExact(v, n) {
  $chunkSize(n);
  const whole = v.length - (v.length % n);
  const chunks = [];
  for (let i = 0; i < whole; i += n) chunks.push(v.slice(i, i + n));
  chunks.remainder = v.slice(whole);
  return chunks;
}

// `rchunks(n)`, and `rchunks_exact(n)`: chunks from the end; the exact ones
// leave what's left at the start, their `remainder()`.
function $rchunks(v, n, exact) {
  $chunkSize(n);
  const chunks = [];
  let end = v.length;
  for (; end >= n; end -= n) chunks.push(v.slice(end - n, end));
  if (exact) chunks.remainder = v.slice(0, end);
  else if (end > 0) chunks.push(v.slice(0, end));
  return chunks;
}

// `split(p)` of a slice: the pieces between the items `p` holds of, at most
// `n`, each with its item where `inclusive`, and from the end where `back`.
function $sliceSplitBy(v, p, n = Infinity, inclusive = false, back = false) {
  if (n === 0) return [];
  const items = back ? [...v].reverse() : v;
  const parts = [];
  let start = 0;
  for (let i = 0; i < items.length && parts.length < n - 1; i++) {
    if (p(items[i])) {
      parts.push(items.slice(start, inclusive ? i + 1 : i));
      start = i + 1;
    }
  }
  if (!inclusive || start < items.length) parts.push(items.slice(start));
  return back ? parts.map((part) => part.reverse()) : parts;
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
function $sliceStrip(v, p, end) {
  if (!$sliceStartsWith(v, p, end)) return undefined;
  return end ? v.slice(0, v.length - p.length) : v.slice(p.length);
}

// `v.repeat(n)`: its items `n` times over, of `Copy` items.
function $repeatItems(v, n) {
  const out = [];
  for (let i = 0; i < n; i++) out.push(...v);
  return out;
}
