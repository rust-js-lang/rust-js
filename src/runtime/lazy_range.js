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
  $forward(range, undefined, {
    nth: (k) => {
      if (k > 0 && at < end) at = Number(at) + k * step;
      return range.next();
    },
  });
  // Its `size_hint`, std's, exact, of how many it has left; of `n..`,
  // `usize::MAX` and none known (ADR 0918).
  range.sizeHint = () => {
    if (end === Infinity) return [4294967295, undefined];
    const left = at < end ? Math.ceil((Number(end) - Number(at)) / step) : 0;
    return [left, left];
  };
  return range;
}

// A range's numbers from its end, `step` apart, made as they're stepped:
// `$lazyRangeBack(0, n, 3)` of `(0..n).rev().step_by(3)`, of `a..=b`
// `inclusive`, its end the first; of numbers, BigInts or `char`s alike.
// Its `size_hint` std's `StepBy`'s of the reversed range it steps: of `n`
// left from the number it gives next down to its start, that one and each
// `step`th after, `1 + (n - 1) / step`, which is std's before its first
// item, `1 + (len - 1) / step`, and after, what its range has left `step`
// apart, so a collect asks the room std's does (#1046).
function $lazyRangeBack(start, end, step, inclusive = false) {
  const chars = typeof end === "string";
  const big = typeof end === "bigint";
  const by = big ? BigInt(step) : step;
  // Of `char`s, by code point, past the surrogates, each step that many
  // back, as std's `Step::backward` (#420).
  let at = chars ? (inclusive ? end : $charStep(end, -1)) : inclusive ? end : end - (big ? 1n : 1);
  const ended = () => (chars ? at === undefined || at.codePointAt(0) < start.codePointAt(0) : at < start);
  const range = new $Steps(() => {
    if (ended()) return { done: true, value: undefined };
    const value = at;
    if (chars) for (let k = 0; k < step && at !== undefined; k++) at = $charStep(at, -1);
    else at -= by;
    return { done: false, value };
  });
  range.sizeHint = () => {
    if (ended()) return [0, 0];
    const [low, high] = $rangeStepsLeft(start, at, true);
    const left = (n) => (n === 0 ? 0 : 1 + Math.floor((n - 1) / step));
    return [left(low), high === undefined ? undefined : left(high)];
  };
  return range;
}

// How many numbers, BigInts or `char`s from `start` to `end` a range has
// left, `inclusive` of its end, both bounds of std's `size_hint`: past
// `usize::MAX`, that and none known.
function $rangeStepsLeft(start, end, inclusive) {
  if (typeof start !== "number") return $stepsLeft(start, end, inclusive);
  const left = Math.max(inclusive ? end - start + 1 : end - start, 0);
  return left > 4294967295 ? [4294967295, undefined] : [left, left];
}
