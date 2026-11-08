// std's stable sort, driftsort (core::slice::sort::stable, ADR 0036), step
// for step, for a sort whose comparison can be seen: `is_less` is asked what
// std's asks, in std's order, and a comparison that panics leaves the items
// where std's guards leave them. `kind` is what std picks by the item's
// type: its small sort, "general8" for a `Freeze` one of at most 16 bytes,
// "general" for a larger `Freeze` one, else "fallback", insertion; `size`
// is its size, which sizes std's scratch (a 32-bit target's, ADR 0090).
function $sortStable(v, isLess, kind, size) {
  const len = v.length;
  // A zero-sized type's items are all equal, so sorted.
  if (size === 0 || len < 2) return;
  if (len <= 20) {
    $insertionSort(v, 0, len, 1, isLess);
    return;
  }
  // `driftsort_main`'s scratch: on the stack, 4 KiB of items, where that
  // holds `max(len - len / 2, min(len, 8 MB of items), 48)`; else that many.
  const full = Math.floor(8_000_000 / size);
  const wanted = Math.max(len - Math.floor(len / 2), Math.min(len, full), 48);
  const stack = Math.floor(4096 / size);
  const sort = { isLess, kind, scratch: stack >= wanted ? stack : wanted, threshold: kind === "fallback" ? 16 : 32 };
  $driftSort(v, 0, len, len <= sort.threshold * 2, sort);
}

// `drift::sort` of `v[start..end]`: runs found or made, merged as
// powersort's tree has them, lazily where both are unsorted and fit the
// scratch, each unsorted one at last quicksorted.
function $driftSort(v, start, end, eager, sort) {
  const len = end - start;
  if (len < 2) return;
  const scale = ((1n << 62n) + BigInt(len) - 1n) / BigInt(len);
  const minGoodRun = len <= 64 * 64 ? Math.min(len - Math.floor(len / 2), 64) : $sqrtApprox(len);
  const runs = [];
  const depths = [];
  let scan = 0;
  let prev = { len: 0, sorted: true };
  for (;;) {
    let next;
    let depth = 0;
    if (scan < len) {
      next = $createRun(v, start + scan, end, minGoodRun, eager, sort);
      depth = $mergeTreeDepth(scan - prev.len, scan, scan + next.len, scale);
    } else {
      next = { len: 0, sorted: true };
    }
    while (runs.length > 1 && depths[depths.length - 1] >= depth) {
      const left = runs.pop();
      depths.pop();
      const merged = left.len + prev.len;
      prev = $logicalMerge(v, start + scan - merged, start + scan, left, prev, sort);
    }
    runs.push(prev);
    depths.push(depth);
    if (scan >= len) break;
    scan += next.len;
    prev = next;
  }
  if (!prev.sorted) $stableQuicksort(v, start, end, sort);
}

// `merge_tree_depth`: the leading zeros of the scaled midpoints' difference.
function $mergeTreeDepth(left, mid, right, scale) {
  const x = scale * BigInt(left + mid);
  const y = scale * BigInt(mid + right);
  const bits = BigInt.asUintN(64, x ^ y);
  return bits === 0n ? 64 : 64 - bits.toString(2).length;
}

function $sqrtApprox(n) {
  const shift = Math.ceil((31 - Math.clz32(n | 1)) / 2);
  return Math.floor((2 ** shift + Math.floor(n / 2 ** shift)) / 2);
}

function $logicalMerge(v, start, end, left, right, sort) {
  const len = end - start;
  if (len > sort.scratch || left.sorted || right.sorted) {
    if (!left.sorted) $stableQuicksort(v, start, start + left.len, sort);
    if (!right.sorted) $stableQuicksort(v, start + left.len, end, sort);
    $mergeRuns(v, start, end, left.len, sort.isLess, sort.scratch);
    return { len, sorted: true };
  }
  return { len, sorted: false };
}

// `create_run` of `v[start..end]`: a run already there, reversed where it
// descends, of at least `minGoodRun`; else, eager, the first
// `threshold` small-sorted; else the first `minGoodRun`, unsorted.
function $createRun(v, start, end, minGoodRun, eager, sort) {
  const len = end - start;
  if (len >= minGoodRun) {
    const [run, reversed] = $existingRun(v, start, end, sort.isLess);
    if (run >= minGoodRun) {
      if (reversed) $reverseRange(v, start, start + run);
      return { len: run, sorted: true };
    }
  }
  if (eager) {
    const run = Math.min(sort.threshold, len);
    $stableQuicksortFrom(v, start, start + run, 0, undefined, sort);
    return { len: run, sorted: true };
  }
  return { len: Math.min(minGoodRun, len), sorted: false };
}

// `stable_quicksort`: at most `2 * floor(log2(len))` bad pivots.
function $stableQuicksort(v, start, end, sort) {
  $stableQuicksortFrom(v, start, end, 2 * (31 - Math.clz32((end - start) | 1)), undefined, sort);
}

// `quicksort` of `v[start..end]`, given its left ancestor's pivot, `[item]`
// or `undefined`: the left side in the loop, the right one recursed into;
// eager driftsort where the pivots have been bad too often. A type that
// isn't `Freeze`, "fallback", passes no pivot on, as std copies none.
function $stableQuicksortFrom(v, start, end, limit, ancestor, sort) {
  const isLess = sort.isLess;
  for (;;) {
    const len = end - start;
    if (len <= sort.threshold) {
      if (sort.kind === "fallback") {
        if (len >= 2) $insertionSort(v, start, end, 1, isLess);
      } else if (len >= 2) {
        $smallSortGeneral(v, start, end, isLess, sort.kind === "general8");
      }
      return;
    }
    if (limit === 0) {
      $driftSort(v, start, end, true, sort);
      return;
    }
    limit--;
    const pivot = $choosePivot(v, start, end, isLess);
    const pivotItem = sort.kind === "fallback" ? undefined : [v[start + pivot]];
    let equal = ancestor !== undefined && !isLess(ancestor[0], v[start + pivot]);
    let left = 0;
    if (!equal) {
      left = $stablePartition(v, start, end, pivot, false, isLess);
      equal = left === 0;
    }
    if (equal) {
      start += $stablePartition(v, start, end, pivot, true, (a, b) => !isLess(b, a));
      ancestor = undefined;
      continue;
    }
    $stableQuicksortFrom(v, start + left, end, limit, pivotItem, sort);
    end = start + left;
  }
}

// `stable_partition` of `v[start..end]` around the item at `start + pivot`,
// which goes left where `pivotGoesLeft`, asked of no comparison: those less
// first, then the rest, each in order. Written into scratch, then back, so
// a comparison that panics leaves `v` as it was. The number less.
function $stablePartition(v, start, end, pivot, pivotGoesLeft, isLess) {
  const len = end - start;
  const p = v[start + pivot];
  const scratch = new Array(len);
  let left = 0;
  let back = len;
  for (let i = 0; i < len; i++) {
    const item = v[start + i];
    const towardsLeft = i === pivot ? pivotGoesLeft : isLess(item, p);
    back--;
    scratch[towardsLeft ? left : back + left] = item;
    if (towardsLeft) left++;
  }
  for (let i = 0; i < left; i++) v[start + i] = scratch[i];
  for (let i = 0; i < len - left; i++) v[start + left + i] = scratch[len - 1 - i];
  return left;
}

// `merge` of the sorted `v[start..start + mid]` and `v[start + mid..end]`:
// the shorter copied out, then merged up or down into the gap. What's left
// of the copy goes back where the gap is, at the end or as a comparison
// panics, as std's `MergeState` writes it.
function $mergeRuns(v, start, end, mid, isLess, scratch) {
  const len = end - start;
  if (mid === 0 || mid >= len || scratch < Math.min(mid, len - mid)) return;
  const leftShorter = mid <= len - mid;
  let dst = leftShorter ? start : start + mid;
  const saved = v.slice(dst, leftShorter ? start + mid : end);
  let from = 0;
  let to = saved.length;
  try {
    if (leftShorter) {
      let right = start + mid;
      while (from !== to && right !== end) {
        const takeLeft = !isLess(v[right], saved[from]);
        v[dst++] = takeLeft ? saved[from++] : v[right++];
      }
    } else {
      let out = end;
      for (;;) {
        const left = dst - 1;
        const right = to - 1;
        out--;
        const takeLeft = isLess(saved[right], v[left]);
        v[out] = takeLeft ? v[left] : saved[right];
        if (takeLeft) dst = left;
        else to = right;
        if (dst === start || to === 0) break;
      }
    }
  } finally {
    for (let i = from; i < to; i++) v[dst + i - from] = saved[i];
  }
}
