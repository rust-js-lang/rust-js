// A slice's fills, copies, order checks, chunks and splits (ADR 0324).

// `v.fill(x)`: a clone of `x` in each but the last, which is `x` itself, as
// std's is: by its type's `clone_from` (`cloneFrom`) where it has one, or
// `clone` where a clone is more than the value. Of items with a destructor,
// each it replaces dropped by `drop`, or `x`, of none to write (ADR 0507);
// `x` dropped too as a clone, or a drop of what it replaces, unwinds, as
// std's `fill` owns it till the last (#898).
function $fill(v, x, clone, cloneFrom, drop) {
  $unwinds(
    () => {
      for (let i = 0; i < v.length - 1; i++) {
        if (cloneFrom) {
          cloneFrom(v, i, x);
          continue;
        }
        const old = v[i];
        v[i] = clone(x);
        drop?.(old);
      }
    },
    () => drop?.(x),
  );
  if (v.length === 0) {
    drop?.(x);
    return;
  }
  const old = v[v.length - 1];
  v[v.length - 1] = x;
  drop?.(old);
}

// `v.fill_with(f)`: each item `f()`'s, made before the one it replaces is
// dropped, `drop`, as `*item = f()` drops it (ADR 0507).
function $fillWith(v, f, drop) {
  for (let i = 0; i < v.length; i++) {
    const made = f();
    drop?.(v[i]);
    v[i] = made;
  }
}

// Of items `copy` copies, a `Copy` struct something changes in place,
// each a copy.
function $copyFromSlice(v, src, copy) {
  if (v.length !== src.length) {
    throw new Error(`copy_from_slice: source slice length (${src.length}) does not match destination slice length (${v.length})`);
  }
  for (let i = 0; i < v.length; i++) v[i] = copy === undefined ? src[i] : copy(src[i]);
}

// `clone_from_slice(src)`: each item a clone, or `cloneFrom(v, i, item)`'s,
// its type's own `clone_from` (ADR 0324).
// `write_clone_of_slice(src)` of `MaybeUninit`s: a clone of each, and the
// slots, panicking as its `assert_eq!` does.
function $writeCloneOfSlice(v, src, clone) {
  if (v.length !== src.length) {
    throw new Error(
      `assertion \`left == right\` failed: destination and source slices have different lengths\n  left: ${v.length}\n right: ${src.length}`,
    );
  }
  for (let i = 0; i < v.length; i++) v[i] = clone ? clone(src[i]) : src[i];
  return v;
}

// `v.clone_from_slice(src)`: each item its type's `clone_from` of the
// source's (`cloneFrom`), or a clone of it, the one it replaces dropped by
// `drop`, as std's assignment drops it (ADR 0507).
function $cloneFromSlice(v, src, clone, cloneFrom, drop) {
  if (v.length !== src.length) throw new Error("destination and source slices have different lengths");
  for (let i = 0; i < v.length; i++) {
    if (cloneFrom) {
      cloneFrom(v, i, src[i]);
      continue;
    }
    const old = v[i];
    v[i] = clone ? clone(src[i]) : src[i];
    drop?.(old);
  }
}

// `w.clone_from(&v)` of a `Vec`, std's `SpecCloneIntoVec`: what `w` has
// past `v`'s length dropped, `truncate`'s; each item it keeps written over
// by `v`'s, `$cloneFromSlice`'s; then a clone of each of the rest pushed,
// what's pushed `w`'s as a later clone panics, `extend_from_slice`'s
// (#919).
function $cloneIntoVec(w, v, clone, cloneFrom, drop) {
  $truncate(w, v.length, drop);
  $cloneFromSlice(w, v.slice(0, w.length), clone, cloneFrom, drop);
  for (let i = w.length; i < v.length; i++) w.push(clone ? clone(v[i]) : v[i]);
}

// `w.clone_from(&v)` of a `VecDeque`, std's: `w` cleared, each item
// dropped, then a clone of each of `v`'s pushed, what's pushed `w`'s as a
// later clone panics (#922).
function $cloneIntoDeque(w, v, clone, drop) {
  $truncate(w, 0, drop);
  for (const item of v) w.push(clone ? clone(item) : item);
}

// `w.clone_from(&v)` of a `LinkedList`, std's: each item `w` keeps written
// over by `v`'s, its type's `clone_from` (`cloneFrom`) or a clone, the one
// it replaces dropped; the rest popped from its back, each dropped, as
// `pop_back` drops it, a panic stopping it; then a clone of each of `v`'s
// rest pushed (#922).
function $cloneIntoList(w, v, clone, cloneFrom, drop) {
  const kept = Math.min(w.length, v.length);
  for (let i = 0; i < kept; i++) {
    if (cloneFrom) {
      cloneFrom(w, i, v[i]);
      continue;
    }
    const old = w[i];
    w[i] = clone ? clone(v[i]) : v[i];
    drop?.(old);
  }
  while (w.length > v.length) {
    const last = w.pop();
    drop?.(last);
  }
  for (let i = kept; i < v.length; i++) w.push(clone ? clone(v[i]) : v[i]);
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

// `is_sorted_by(f)` of a slice's or an iterator's items: `f` of each two
// in a row, `true` for all. It stops at the first `false`, as Rust's does,
// so an iterator's later items aren't made. Given `ends`, what drops with
// `f`: run at its end and as it unwinds, as std's `all` drops `f` before
// what's left of the iterator (#763).
//
// Given `drop`, of items with a destructor, the keys `is_sorted_by_key`
// makes: stepped as std's `is_sorted_by` steps them, each item kept as
// `last` till the next replaces it, and dropped as `*last = curr` drops
// it, `last` then `curr` though that drop panics. Where two are out of
// order, `curr` dropped, then what drops with `f`, as `all` ends, then
// `last`, then what's left of `items` closed, as `is_sorted_by`'s frame
// drops them, and so as `f` or a step panics (#776).
function $isSortedBy(items, f, ends, drop) {
  if (drop !== undefined) {
    const it = Iterator.from(items);
    let last;
    let holding = false;
    let done = false;
    const end = () =>
      $inTurn(
        ends,
        () => holding && ((holding = false), drop(last)),
        () => done || it.return?.(),
      );
    let sorted = true;
    try {
      let step = it.next();
      if (!step.done) {
        last = step.value;
        holding = true;
        while (!(step = it.next()).done) {
          const item = step.value;
          let ordered;
          try {
            ordered = f(last, item);
          } catch (error) {
            drop(item);
            throw error;
          }
          if (!ordered) {
            drop(item);
            sorted = false;
            break;
          }
          const replaced = last;
          last = item;
          drop(replaced);
        }
      }
      done = step.done;
    } catch (error) {
      try {
        end();
      } catch {}
      throw error;
    }
    end();
    return sorted;
  }
  // Stepped by hand, closed where its step throws, a panic, as `for..of`
  // doesn't, and where it stops early, as `is_sorted_by(self)` drops what
  // it took (#1126).
  const it = typeof items.next === "function" ? items : items[Symbol.iterator]();
  let first = true;
  let last;
  try {
    return $unwinds(
      () => {
        for (let step = it.next(); !step.done; step = it.next()) {
          if (!first && !f(last, step.value)) {
            it.return?.();
            return false;
          }
          first = false;
          last = step.value;
        }
        return true;
      },
      () => it.return?.(),
    );
  } finally {
    ends?.();
  }
}

// `is_sorted_by_key(f, cmp)`: `f` of each item once, as an iterator's is, up
// to the first out of order. Of keys with a destructor, given their
// `drop`: std's `self.iter().is_sorted_by_key(f)`, each key dropped where
// `$isSortedBy` drops it (#783).
function $isSortedByKey(v, f, cmp, drop) {
  if (drop !== undefined)
    return $isSortedBy(
      v.values().map((item) => f(item)),
      (a, b) => cmp(a, b) <= 0,
      undefined,
      drop,
    );
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

// `s.split_off(..n)`, or `split_off(n..)` (`back`): what's left of `s` and
// what's taken off it, or `s` and `undefined`, `None`, where `n` is past
// its end (ADR 0338).
function $sliceSplitOff(v, n, back = false, cut = $copyOf) {
  if (n > v.length) return [v, undefined];
  return back ? [cut(v, 0, n), cut(v, n, v.length)] : [cut(v, n, v.length), cut(v, 0, n)];
}

// `split_off_first()`, or `split_off_last()` (`last`): what's left and the
// item taken off, or `s` and `undefined` of an empty one.
function $sliceSplitOffEnd(v, last, cut = $copyOf, item = (items, i) => items[i]) {
  if (v.length === 0) return [v, undefined];
  return last ? [cut(v, 0, v.length - 1), item(v, v.length - 1)] : [cut(v, 1, v.length), item(v, 0)];
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
function $sortByCachedKey(v, f, cmp, kind) {
  const keyed = v.map((item, i) => [f(item), i, item]);
  // Where the keys' comparisons can be seen, std's unstable sort of the
  // `(key, index)` pairs, by the tuple's `lt`: the keys' `partial_cmp`,
  // then the indexes where they're equal (#925).
  if (kind !== undefined) {
    $sortUnstable(keyed, (a, b) => {
      const order = cmp(a[0], b[0]);
      return order === 0 ? a[1] < b[1] : order < 0;
    }, kind);
  } else {
    keyed.sort((a, b) => cmp(a[0], b[0]) || a[1] - b[1]);
  }
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

// `v.repeat(n)`: its items `n` times over, of `Copy` items, room for them
// of `size` bytes each asked first (ADR 0774).
function $repeatItems(v, n, size, copy) {
  $roomFor(v.length * n, size);
  const out = [];
  for (let i = 0; i < n; i++) $extend(out, copy === undefined ? v : v.map(copy));
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
// on a number or a string, whose change is kept; each it doesn't, given to
// `drop` as it goes, as std drops it (ADR 0499); where one throws, what's
// left as `$retain` leaves it (#416).
function $retainMut(v, f, handles, drop) {
  let kept = 0;
  let i = 0;
  try {
    while (i < v.length) {
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
      i++;
      if (keep) v[kept++] = v[at];
      else drop?.(v[at]);
    }
  } finally {
    const rest = v.length - i;
    if (rest > 0) v.copyWithin(kept, i);
    v.length = kept + rest;
  }
}

// A `VecDeque`'s `retain_mut(f)`, and its `retain`, std's: each item `f`
// keeps swapped forward into its place, given as itself, or as a handle on
// a number or a string (`handles`); once every one is asked, what's past
// those kept truncated, each dropped by `drop` in the order the swaps left
// them. Where `f` panics, the deque as the swaps left it, every item in it,
// as std's has no guard (#909).
function $retainDeque(v, f, handles, drop) {
  const keeps = (at) =>
    handles
      ? f({
          get value() {
            return v[at];
          },
          set value(item) {
            v[at] = item;
          },
        })
      : f(v[at]);
  const len = v.length;
  let kept = 0;
  let at = 0;
  while (at < len) {
    if (!keeps(at++)) break;
    kept++;
  }
  while (at < len) {
    if (!keeps(at)) {
      at++;
      continue;
    }
    [v[kept], v[at]] = [v[at], v[kept]];
    at++;
    kept++;
  }
  if (at !== kept) $truncate(v, kept, drop);
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

// A deque's `push_front_mut(x)`: `x` put first, and a `&mut` to it.
function $pushFrontMut(v, x, handle) {
  v.unshift(x);
  return handle ? $mutAt(v, 0) : x;
}

// A `VecDeque`'s `insert(i, x)` and `insert_mut`: checked as its
// `assert!` checks, and a `&mut` to what it put in.
function $dequeInsert(v, i, x, handle) {
  if (i > v.length) throw new Error("index out of bounds");
  v.splice(i, 0, x);
  return handle ? $mutAt(v, i) : x;
}

function $insertMut(v, i, x, handle) {
  $insertAt(v, i, x);
  return handle ? $mutAt(v, i) : x;
}
