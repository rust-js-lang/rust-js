// What a double-ended iterator does from its back, of a JS iterator that
// knows its back, a `$iter` or a `$iterator` given its `nextBack`, as Rust's
// `next_back` steps it (ADR 0429).

// `it.nth_back(n)`: its `n`th item from the back, each before it stepped
// past, and dropped, `drop`, where items have a destructor, or `undefined`,
// `None`; of items that could look like `None`, its `Some`, `$some`, given
// as `found` (ADR 0051).
function $nthBack(it, n, found = (item) => item, drop) {
  for (let step = it.back(); !step.done; step = it.back()) {
    if (n-- === 0) return found(step.value);
    drop?.(step.value);
  }
  return undefined;
}

// `it.rfold(init, f)`: `f` of each item from the back, given what it gave.
function $foldBack(it, init, f) {
  let folded = init;
  for (let step = it.back(); !step.done; step = it.back()) folded = f(folded, step.value);
  return folded;
}

// `it.rfind(p)`: the first item from the back `p` holds of, or `undefined`;
// `found` and `drop`, of each it passes over, as `$nthBack`'s.
function $findBack(it, p, found = (item) => item, drop) {
  for (let step = it.back(); !step.done; step = it.back()) {
    if (p(step.value)) return found(step.value);
    drop?.(step.value);
  }
  return undefined;
}

// A map of a JS iterator that knows its back or its length, `it.map(f)`,
// which knows them too, as Rust's `Map` is double-ended and exact-sized
// where its source is: each of its source's items, `f`'s, from either end
// (ADRs 0455, 0456).
function $mapEnds(it, f) {
  const mapped = (step) => (step.done ? step : { done: false, value: f(step.value) });
  const map = Iterator.from({ next: () => mapped(it.next()) });
  if (it.back !== undefined) map.back = () => mapped(it.back());
  if (it.len !== undefined) map.len = () => it.len();
  return map;
}

// A filter of a JS iterator that knows its back, `it.filter(p)`, which
// knows it too, as Rust's `Filter` is double-ended where its source is:
// each of its source's items `p` keeps, from either end, `p` asked of each
// as it's reached (ADR 0526).
function $filterEnds(it, p) {
  const kept = (step) => () => {
    for (let at = step(); !at.done; at = step()) if (p(at.value)) return at;
    return { done: true, value: undefined };
  };
  const filter = Iterator.from({ next: kept(() => it.next()) });
  if (it.back !== undefined) filter.back = kept(() => it.back());
  return filter;
}

// `it.enumerate()` of a JS iterator that knows its back or its length,
// which knows them too, as Rust's `Enumerate` does: each item and its
// index, `[i, x]`, counted from the front; one from the back is where it
// is, the count so far and how many were left before it, as Rust's
// `next_back` counts it (ADR 0528).
function $enumerateEnds(it) {
  let count = 0;
  const pair = (step, at) => (step.done ? step : { done: false, value: [at, step.value] });
  const enumerated = Iterator.from({
    next: () => {
      const step = it.next();
      return step.done ? step : pair(step, count++);
    },
  });
  if (it.back !== undefined && it.len !== undefined) {
    enumerated.back = () => {
      const left = it.len();
      return pair(it.back(), count + left - 1);
    };
  }
  if (it.len !== undefined) enumerated.len = () => it.len();
  return enumerated;
}
