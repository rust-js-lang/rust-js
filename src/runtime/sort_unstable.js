// std's unstable sort, `ipnsort`, and `select_nth_unstable`'s introselect
// (core::slice::sort, ADR 0342), step for step: items `is_less` doesn't
// order end where std's leave them, and `is_less` is asked what std's asks,
// in std's order. `kind` is what std picks by the item's type: its small
// sort, "network" for a `Copy`, `Freeze` one of at most 8 bytes, "general"
// for a `Freeze` one of at most 85, "general8" of at most 16, or else
// "fallback"; "hoare" partitions one of more than 96 bytes.
function $sortUnstable(v, isLess, kind) {
  const len = v.length;
  if (len < 2) return;
  if (len <= 20) {
    $insertionSort(v, 0, len, 1, isLess);
    return;
  }
  const [runLength, reversed] = $existingRun(v, isLess);
  if (runLength === len) {
    if (reversed) v.reverse();
    return;
  }
  $quicksort(v, 0, len, undefined, 2 * Math.floor(Math.log2(len | 1)), isLess, kind);
}

// `select_nth_unstable(index)`: the item at `index` where a sort would put
// it, those before it not after it and those after not before; then views
// of those before and after, and it, a handle on a number or text
// (`handle`).
function $selectNthUnstable(v, index, isLess, kind, handle = false) {
  const len = v.length;
  if (index >= len) throw new Error(`partition_at_index index ${index} greater than length of slice ${len}`);
  if (index === len - 1) $swapItems(v, $extremeIndex(v, 0, len, isLess, true), index);
  else if (index === 0) $swapItems(v, $extremeIndex(v, 0, len, isLess, false), index);
  else $selectLoop(v, 0, len, index, isLess, kind);
  return [$view(v, 0, index), handle ? $mutAt(v, index) : v[index], $view(v, index + 1)];
}

function $swapItems(v, a, b) {
  const item = v[a];
  v[a] = v[b];
  v[b] = item;
}

// `insert_tail` of each from `offset` on, of `v[start..end]`.
function $insertionSort(v, start, end, offset, isLess) {
  for (let tail = start + offset; tail < end; tail++) {
    let sift = tail - 1;
    if (!isLess(v[tail], v[sift])) continue;
    const item = v[tail];
    let gap = tail;
    for (;;) {
      v[gap] = v[sift];
      gap = sift;
      if (sift === start) break;
      sift--;
      if (!isLess(item, v[sift])) break;
    }
    v[gap] = item;
  }
}

function $existingRun(v, isLess) {
  const len = v.length;
  let run = 2;
  const descending = isLess(v[1], v[0]);
  if (descending) while (run < len && isLess(v[run], v[run - 1])) run++;
  else while (run < len && !isLess(v[run], v[run - 1])) run++;
  return [run, descending];
}

function $quicksort(v, start, end, ancestor, limit, isLess, kind) {
  const threshold = kind === "network" || kind === "general" || kind === "general8" ? 32 : 16;
  for (;;) {
    const len = end - start;
    if (len <= threshold) {
      $smallSort(v, start, end, isLess, kind);
      return;
    }
    if (limit === 0) {
      $heapsort(v, start, end, isLess);
      return;
    }
    limit--;
    const pivot = $choosePivot(v, start, end, isLess);
    if (ancestor !== undefined && !isLess(v[ancestor], v[start + pivot])) {
      const lt = $sortPartition(v, start, end, pivot, (a, b) => !isLess(b, a), kind);
      start += lt + 1;
      ancestor = undefined;
      continue;
    }
    const lt = $sortPartition(v, start, end, pivot, isLess, kind);
    $quicksort(v, start, start + lt, ancestor, limit, isLess, kind);
    ancestor = start + lt;
    start += lt + 1;
  }
}

function $selectLoop(v, start, end, index, isLess, kind) {
  let limit = 16;
  let ancestor;
  for (;;) {
    if (end - start <= 16) {
      if (end - start >= 2) $insertionSort(v, start, end, 1, isLess);
      return;
    }
    if (limit === 0) {
      $medianOfMedians(v, start, end, index, isLess, kind);
      return;
    }
    limit--;
    const pivot = $choosePivot(v, start, end, isLess);
    if (ancestor !== undefined && !isLess(v[ancestor], v[start + pivot])) {
      const mid = $sortPartition(v, start, end, pivot, (a, b) => !isLess(b, a), kind) + 1;
      if (mid > index) return;
      start += mid;
      index -= mid;
      ancestor = undefined;
      continue;
    }
    const mid = $sortPartition(v, start, end, pivot, isLess, kind);
    if (mid < index) {
      ancestor = start + mid;
      start += mid + 1;
      index -= mid + 1;
    } else if (mid > index) {
      end = start + mid;
    } else {
      return;
    }
  }
}

// `min_index`, or `max_index` (`max`), of `v[start..end]`, from `start`.
function $extremeIndex(v, start, end, isLess, max) {
  let at = start;
  for (let i = start + 1; i < end; i++) if (max ? isLess(v[at], v[i]) : isLess(v[i], v[at])) at = i;
  return at;
}

function $medianOfMedians(v, start, end, k, isLess, kind) {
  for (;;) {
    const len = end - start;
    if (len <= 16) {
      if (len >= 2) $insertionSort(v, start, end, 1, isLess);
      return;
    }
    if (k === len - 1) {
      $swapItems(v, $extremeIndex(v, start, end, isLess, true), start + k);
      return;
    }
    if (k === 0) {
      $swapItems(v, $extremeIndex(v, start, end, isLess, false), start);
      return;
    }
    const p = $medianOfNinthers(v, start, end, isLess, kind);
    if (p === k) return;
    if (p > k) end = start + p;
    else {
      start += p + 1;
      k -= p + 1;
    }
  }
}

function $medianOfNinthers(v, start, end, isLess, kind) {
  const len = end - start;
  const frac = len <= 1024 ? Math.floor(len / 12) : len <= 128 * 1024 ? Math.floor(len / 64) : Math.floor(len / 1024);
  const pivot = Math.floor(frac / 2);
  const lo = Math.floor(len / 2) - pivot;
  const hi = frac + lo;
  const gap = Math.floor((len - 9 * frac) / 4);
  let a = lo - 4 * frac - gap;
  let b = hi + gap;
  for (let i = lo; i < hi; i++) {
    $ninther(v, start, isLess, a, i - frac, b, a + 1, i, b + 1, a + 2, i + frac, b + 2);
    a += 3;
    b += 3;
  }
  $medianOfMedians(v, start + lo, start + lo + frac, pivot, isLess, kind);
  return $sortPartition(v, start, end, lo + pivot, isLess, kind);
}

function $ninther(v, s, isLess, a, b, c, d, e, f, g, h, i) {
  const at = (x) => v[s + x];
  b = $medianIndex(v, s, isLess, a, b, c);
  h = $medianIndex(v, s, isLess, g, h, i);
  if (isLess(at(h), at(b))) [b, h] = [h, b];
  if (isLess(at(f), at(d))) [d, f] = [f, d];
  if (isLess(at(e), at(d))) {
    // `d` stays.
  } else if (isLess(at(f), at(e))) {
    d = f;
  } else {
    if (isLess(at(e), at(b))) $swapItems(v, s + e, s + b);
    else if (isLess(at(h), at(e))) $swapItems(v, s + e, s + h);
    return;
  }
  if (isLess(at(d), at(b))) d = b;
  else if (isLess(at(h), at(d))) d = h;
  $swapItems(v, s + d, s + e);
}

function $medianIndex(v, s, isLess, a, b, c) {
  if (isLess(v[s + c], v[s + a])) [a, c] = [c, a];
  if (isLess(v[s + c], v[s + b])) return c;
  if (isLess(v[s + b], v[s + a])) return a;
  return b;
}

// `choose_pivot` of `v[start..end]`: its index, from `start`.
function $choosePivot(v, start, end, isLess) {
  const len = end - start;
  const n8 = Math.floor(len / 8);
  const [a, b, c] = [start, start + n8 * 4, start + n8 * 7];
  return (len < 64 ? $median3(v, a, b, c, isLess) : $median3Rec(v, a, b, c, n8, isLess)) - start;
}

function $median3Rec(v, a, b, c, n, isLess) {
  if (n * 8 >= 64) {
    const n8 = Math.floor(n / 8);
    a = $median3Rec(v, a, a + n8 * 4, a + n8 * 7, n8, isLess);
    b = $median3Rec(v, b, b + n8 * 4, b + n8 * 7, n8, isLess);
    c = $median3Rec(v, c, c + n8 * 4, c + n8 * 7, n8, isLess);
  }
  return $median3(v, a, b, c, isLess);
}

function $median3(v, a, b, c, isLess) {
  const x = isLess(v[a], v[b]);
  const y = isLess(v[a], v[c]);
  if (x === y) return isLess(v[b], v[c]) !== x ? c : b;
  return a;
}

function $heapsort(v, start, end, isLess) {
  const len = end - start;
  for (let i = len + Math.floor(len / 2) - 1; i >= 0; i--) {
    let node = i;
    if (i >= len) node = i - len;
    else {
      $swapItems(v, start, start + i);
      node = 0;
    }
    const size = Math.min(i, len);
    for (;;) {
      let child = 2 * node + 1;
      if (child >= size) break;
      if (child + 1 < size && isLess(v[start + child], v[start + child + 1])) child++;
      if (!isLess(v[start + node], v[start + child])) break;
      $swapItems(v, start + node, start + child);
      node = child;
    }
  }
}

// `partition` of `v[start..end]` around the item at `start + pivot`: the
// number less than it, which then stands there.
function $sortPartition(v, start, end, pivot, isLess, kind) {
  $swapItems(v, start, start + pivot);
  const lt = kind === "hoare" ? $partitionHoare(v, start, end, isLess) : $partitionLomuto(v, start, end, isLess);
  $swapItems(v, start, start + lt);
  return lt;
}

// `partition_lomuto_branchless_cyclic` of `v[start + 1..end]` around
// `v[start]`: the item first taken out goes back in last.
function $partitionLomuto(v, start, end, isLess) {
  const base = start + 1;
  const len = end - base;
  if (len === 0) return 0;
  const pivot = v[start];
  const taken = v[base];
  let lt = 0;
  let gap = base;
  const step = (right) => {
    const item = right === undefined ? taken : v[right];
    const rightIsLt = isLess(item, pivot);
    v[gap] = v[base + lt];
    v[base + lt] = item;
    gap = right;
    lt += rightIsLt ? 1 : 0;
  };
  for (let right = base + 1; right < end; right++) step(right);
  step(undefined);
  return lt;
}

// `partition_hoare_branchy_cyclic` of `v[start + 1..end]` around `v[start]`.
function $partitionHoare(v, start, end, isLess) {
  const base = start + 1;
  const pivot = v[start];
  if (end === base) return 0;
  let left = base;
  let right = end;
  let gap;
  for (;;) {
    while (left < right && isLess(v[left], pivot)) left++;
    for (;;) {
      right--;
      if (left >= right || isLess(v[right], pivot)) break;
    }
    if (left >= right) break;
    if (gap === undefined) gap = { at: right, item: v[left] };
    else v[gap.at] = v[left];
    gap.at = right;
    v[left] = v[right];
    left++;
  }
  if (gap !== undefined) v[gap.at] = gap.item;
  return left - base;
}

function $smallSort(v, start, end, isLess, kind) {
  const len = end - start;
  if (len < 2) return;
  if (kind === "network") $smallSortNetwork(v, start, end, isLess);
  else if (kind === "general" || kind === "general8") $smallSortGeneral(v, start, end, isLess, kind === "general8");
  else $insertionSort(v, start, end, 1, isLess);
}

function $smallSortNetwork(v, start, end, isLess) {
  const len = end - start;
  const half = Math.floor(len / 2);
  const noMerge = len < 18;
  const regions = noMerge ? [[start, end]] : [[start, start + half], [start + half, end]];
  for (const [from, to] of regions) {
    const size = to - from;
    let presorted = 1;
    if (size >= 13) {
      $sortNetwork(v, from, $SORT13, isLess);
      presorted = 13;
    } else if (size >= 9) {
      $sortNetwork(v, from, $SORT9, isLess);
      presorted = 9;
    }
    $insertionSort(v, from, to, presorted, isLess);
  }
  if (noMerge) return;
  const merged = new Array(len);
  $bidirectionalMerge(v, start, len, merged, 0, isLess);
  for (let i = 0; i < len; i++) v[start + i] = merged[i];
}

const $SORT9 = [0, 3, 1, 7, 2, 5, 4, 8, 0, 7, 2, 4, 3, 8, 5, 6, 0, 2, 1, 3, 4, 5, 7, 8, 1, 4, 3, 6, 5, 7, 0, 1, 2, 4, 3, 5, 6, 8, 2, 3, 4, 5, 6, 7, 1, 2, 3, 4, 5, 6];
const $SORT13 = [
  0, 12, 1, 10, 2, 9, 3, 7, 5, 11, 6, 8, 1, 6, 2, 3, 4, 11, 7, 9, 8, 10, 0, 4, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 4, 6, 5, 9, 8, 11,
  10, 12, 0, 5, 3, 8, 4, 7, 6, 11, 9, 10, 0, 1, 2, 5, 6, 9, 7, 8, 10, 11, 1, 3, 2, 4, 5, 6, 9, 10, 1, 2, 3, 4, 5, 7, 6, 8, 2, 3,
  4, 5, 6, 7, 8, 9, 3, 4, 5, 6,
];

// Each pair's `swap_if_less`: the second before the first where it's less.
function $sortNetwork(v, from, pairs, isLess) {
  for (let i = 0; i < pairs.length; i += 2) {
    const [a, b] = [from + pairs[i], from + pairs[i + 1]];
    if (isLess(v[b], v[a])) $swapItems(v, a, b);
  }
}

function $smallSortGeneral(v, start, end, isLess, sort8) {
  const len = end - start;
  const half = Math.floor(len / 2);
  const scratch = new Array(len + 16);
  let presorted;
  if (sort8 && len >= 16) {
    $sort8Stable(v, start, scratch, 0, scratch, len, isLess);
    $sort8Stable(v, start + half, scratch, half, scratch, len + 8, isLess);
    presorted = 8;
  } else if (len >= 8) {
    $sort4Stable(v, start, scratch, 0, isLess);
    $sort4Stable(v, start + half, scratch, half, isLess);
    presorted = 4;
  } else {
    scratch[0] = v[start];
    scratch[half] = v[start + half];
    presorted = 1;
  }
  for (const offset of [0, half]) {
    const desired = offset === 0 ? half : len - half;
    for (let i = presorted; i < desired; i++) {
      scratch[offset + i] = v[start + offset + i];
      $insertionSort(scratch, offset, offset + i + 1, i, isLess);
    }
  }
  $bidirectionalMerge(scratch, 0, len, v, start, isLess);
}

// `sort4_stable` of `src[from..from + 4]` into `dst[at..at + 4]`.
function $sort4Stable(src, from, dst, at, isLess) {
  const c1 = isLess(src[from + 1], src[from]);
  const c2 = isLess(src[from + 3], src[from + 2]);
  const a = from + (c1 ? 1 : 0);
  const b = from + (c1 ? 0 : 1);
  const c = from + 2 + (c2 ? 1 : 0);
  const d = from + 2 + (c2 ? 0 : 1);
  const c3 = isLess(src[c], src[a]);
  const c4 = isLess(src[d], src[b]);
  const min = c3 ? c : a;
  const max = c4 ? b : d;
  const unknownLeft = c3 ? a : c4 ? c : b;
  const unknownRight = c4 ? d : c3 ? b : c;
  const c5 = isLess(src[unknownRight], src[unknownLeft]);
  const lo = c5 ? unknownRight : unknownLeft;
  const hi = c5 ? unknownLeft : unknownRight;
  [dst[at], dst[at + 1], dst[at + 2], dst[at + 3]] = [src[min], src[lo], src[hi], src[max]];
}

// `sort8_stable`: two `sort4_stable`s into `scratch[from..]`, merged into `dst`.
function $sort8Stable(src, from, dst, at, scratch, scratchAt, isLess) {
  $sort4Stable(src, from, scratch, scratchAt, isLess);
  $sort4Stable(src, from + 4, scratch, scratchAt + 4, isLess);
  $bidirectionalMerge(scratch, scratchAt, 8, dst, at, isLess);
}

// `bidirectional_merge` of the sorted halves of `src[from..from + len]`
// into `dst[at..]`, from both ends at once.
function $bidirectionalMerge(src, from, len, dst, at, isLess) {
  const half = Math.floor(len / 2);
  let [left, right, out] = [from, from + half, at];
  let [leftRev, rightRev, outRev] = [from + half - 1, from + len - 1, at + len - 1];
  for (let i = 0; i < half; i++) {
    const up = !isLess(src[right], src[left]);
    dst[out++] = src[up ? left : right];
    if (up) left++;
    else right++;
    const down = !isLess(src[rightRev], src[leftRev]);
    dst[outRev--] = src[down ? rightRev : leftRev];
    if (down) rightRev--;
    else leftRev--;
  }
  const [leftEnd, rightEnd] = [leftRev + 1, rightRev + 1];
  if (len % 2 !== 0) {
    const fromLeft = left < leftEnd;
    dst[out] = src[fromLeft ? left : right];
    if (fromLeft) left++;
    else right++;
  }
  if (left !== leftEnd || right !== rightEnd) {
    throw new Error("user-provided comparison function does not correctly implement a total order");
  }
}
