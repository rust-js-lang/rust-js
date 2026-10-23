// A range's numbers, made as they're stepped, `step` apart: `$lazyRange(0,
// n)` of `0..n`, `$lazyRange(0, n, 3)` of `(0..n).step_by(3)`.
function* $lazyRange(start, end = Infinity, step = 1) {
  for (let i = start; i < end; i = Number(i) + step) {
    yield i;
  }
}

// A range's numbers from its end, `step` apart, made as they're stepped:
// `$lazyRangeBack(0, n, 3)` of `(0..n).rev().step_by(3)`, of `a..=b`
// `inclusive`, its end the first; of numbers or BigInts alike.
function* $lazyRangeBack(start, end, step, inclusive = false) {
  const big = typeof end === "bigint";
  const by = big ? BigInt(step) : step;
  for (let i = inclusive ? end : end - (big ? 1n : 1); i >= start; i -= by) {
    yield i;
  }
}
