// A range's numbers, made as they're stepped, `step` apart: `$lazyRange(0,
// n)` of `0..n`, `$lazyRange(0, n, 3)` of `(0..n).step_by(3)`.
function* $lazyRange(start, end = Infinity, step = 1) {
  for (let i = start; i < end; i = Number(i) + step) {
    yield i;
  }
}

// How many numbers a range has, `step` apart, without stepping them, as
// std's `count()` of a range does.
function $lazyRangeLen(start, end, step = 1) {
  return end > start ? Math.ceil((end - start) / step) : 0;
}

// Its last number, `last()`'s, or `undefined` of none.
function $lazyRangeLast(start, end, step = 1) {
  return end > start ? start + ($lazyRangeLen(start, end, step) - 1) * step : undefined;
}
