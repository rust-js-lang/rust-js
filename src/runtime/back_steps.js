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

// A map of a double-ended iterator, `it.map(f)`, which knows its back: each
// of its source's items, `f`'s, from either end, as Rust's `Map` steps them.
function $mapBack(it, f) {
  const mapped = (step) => (step.done ? step : { done: false, value: f(step.value) });
  const map = Iterator.from({ next: () => mapped(it.next()) });
  map.back = () => mapped(it.back());
  return map;
}
