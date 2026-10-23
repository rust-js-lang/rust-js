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

// `it` closed, `return()`, where it can be, as an adapter closed closes
// what it steps (ADR 0465): done.
function $closed(it) {
  it.return?.();
  return { done: true, value: undefined };
}

// A map of a JS iterator that knows its back or its length, `it.map(f)`,
// which knows them too, as Rust's `Map` is double-ended and exact-sized
// where its source is: each of its source's items, `f`'s, from either end
// (ADRs 0455, 0456). Closing it closes its source (ADR 0465).
function $mapEnds(it, f) {
  const mapped = (step) => (step.done ? step : { done: false, value: f(step.value) });
  const map = Iterator.from({ next: () => mapped(it.next()), return: () => $closed(it) });
  if (it.back !== undefined) map.back = () => mapped(it.back());
  if (it.len !== undefined) map.len = () => it.len();
  return map;
}

// A chain of two JS iterators that know their backs, `a.chain(b)`, which
// knows its own, as Rust's `Chain` is double-ended where both are: `a`'s
// then `b`'s from the front, `b`'s then `a`'s from the back, each let go of
// once it's done, as std's clears it. Closing it closes both (ADR 0465).
function $chainEnds(a, b) {
  const done = { done: true, value: undefined };
  const sides = { a, b };
  const step = (first, second, by) => () => {
    for (const side of [first, second]) {
      if (sides[side] === undefined) continue;
      const at = sides[side][by]();
      if (!at.done) return at;
      sides[side] = undefined;
    }
    return done;
  };
  const chain = Iterator.from({
    next: step("a", "b", "next"),
    return: () => {
      sides.a?.return?.();
      sides.b?.return?.();
      return done;
    },
  });
  chain.back = step("b", "a", "back");
  return chain;
}

// `it.step_by(step)` of a JS iterator that knows its back and its length,
// which knows its back too, as Rust's `StepBy` is double-ended where its
// source is exact-sized: the first item, then each `step`th, from the
// front; from the back, the last of those, found by the source's length,
// as std's `next_back_index` finds it. Closing it closes its source.
function $stepByEnds(it, step) {
  let first = true;
  const skip = (k, by) => {
    for (; k > 0; k--) {
      const at = it[by]();
      if (at.done) return at;
    }
    return it[by]();
  };
  const stepped = Iterator.from({
    next() {
      const at = skip(first ? 0 : step - 1, "next");
      first = false;
      return at;
    },
    return: () => $closed(it),
  });
  stepped.back = () => {
    const rem = it.len() % step;
    return skip(first ? (rem === 0 ? step - 1 : rem - 1) : rem, "back");
  };
  return stepped;
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

// `it.enumerate()`, Rust's `Enumerate`, explicit (ADR 0542): what it
// steps, `iter`, and how many it's given, `count`, kept, which its size
// hint, its `Debug` and a clone read. Each item and its index, `[i, x]`,
// counted from the front; one from the back is where it is, the count so
// far and how many were left before it, as Rust's `next_back` counts it
// (ADR 0528). Its back and its length are its source's, asked as they're
// asked: a use site may give a source its own, `$knowing`'s. A clone starts
// at the `count` it's given; closing it closes what it steps.
function $enumerate(iter, count = 0) {
  const pair = (step, at) => (step.done ? step : { done: false, value: [at, step.value] });
  const enumerated = Iterator.from({
    next: () => {
      const step = enumerated.iter.next();
      return step.done ? step : pair(step, enumerated.count++);
    },
    return: () => $closed(enumerated.iter),
  });
  enumerated.iter = iter;
  enumerated.count = count;
  enumerated.hint = () => $sizeHint(enumerated.iter);
  enumerated.back = () => {
    const left = enumerated.iter.len();
    return pair(enumerated.iter.back(), enumerated.count + left - 1);
  };
  enumerated.len = () => enumerated.iter.len();
  return enumerated;
}
