// A range's numbers, made as they're stepped, `step` apart: `$lazyRange(0,
// n)` of `0..n`, `$lazyRange(0, n, 3)` of `(0..n).step_by(3)`. Its `nth`
// is std's `Range::nth`, of its bounds, which `skip` and `step_by` step it
// by, so they don't make each number (#231).
function $lazyRange(start, end = Infinity, step = 1) {
  let at = start;
  const range = new $Steps(() => {
    if (!(at < end)) return { done: true, value: undefined };
    const value = at;
    at = Number(at) + step;
    return { done: false, value };
  });
  range.nth = (k) => {
    if (k > 0 && at < end) at = Number(at) + k * step;
    return range.next();
  };
  // Its `size_hint`, std's, exact, of how many it has left; of `n..`,
  // `usize::MAX` and none known (ADR 0918).
  range.hint = () => {
    if (end === Infinity) return [4294967295, undefined];
    const left = at < end ? Math.ceil((Number(end) - Number(at)) / step) : 0;
    return [left, left];
  };
  return range;
}

// A range's numbers from its end, `step` apart, made as they're stepped:
// `$lazyRangeBack(0, n, 3)` of `(0..n).rev().step_by(3)`, of `a..=b`
// `inclusive`, its end the first; of numbers, BigInts or `char`s alike.
function* $lazyRangeBack(start, end, step, inclusive = false) {
  // Of `char`s, by code point, past the surrogates, each step that many
  // back, as std's `Step::backward` (#420).
  if (typeof end === "string") {
    let c = inclusive ? end : $charStep(end, -1);
    while (c !== undefined && c.codePointAt(0) >= start.codePointAt(0)) {
      yield c;
      for (let k = 0; k < step && c !== undefined; k++) c = $charStep(c, -1);
    }
    return;
  }
  const big = typeof end === "bigint";
  const by = big ? BigInt(step) : step;
  for (let i = inclusive ? end : end - (big ? 1n : 1); i >= start; i -= by) {
    yield i;
  }
}
