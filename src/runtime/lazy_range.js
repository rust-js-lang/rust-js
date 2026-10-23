// A range's numbers, made as they're stepped, `step` apart: `$lazyRange(0,
// n)` of `0..n`, `$lazyRange(0, n, 3)` of `(0..n).step_by(3)`.
function* $lazyRange(start, end = Infinity, step = 1) {
  for (let i = start; i < end; i = Number(i) + step) {
    yield i;
  }
}
