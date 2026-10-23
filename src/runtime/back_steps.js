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
  const map = new $Steps(
    () => mapped(it.next()),
    () => $closed(it),
  );
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
  // The first side let go of once it's done, as std's clears it; the
  // second stepped on, as std's is.
  const step = (first, second, by) => () => {
    if (sides[first] !== undefined) {
      const at = sides[first][by]();
      if (!at.done) return at;
      sides[first] = undefined;
    }
    return sides[second] === undefined ? done : sides[second][by]();
  };
  const chain = new $Steps(step("a", "b", "next"), () => {
    sides.a?.return?.();
    sides.b?.return?.();
  });
  chain.back = step("b", "a", "back");
  return chain;
}

// `it.step_by(step)`, as Rust's `StepBy` steps: its first item, then
// each `step`th, a skip stopped by a `None`, which it gives, and stepped on
// after, as std's `nth` is; each passed over dropped, `drop`, where items
// have a destructor (ADR 0465). Closing it closes its source. Its `nth`,
// `fold` and `tryFold` are std's: its first item its source's `next`, then
// each its source's `nth` (#182).
function $stepBy(it, step, drop) {
  const skip = (k, by) => {
    // From the front, its source's `nth`, as std's `StepBy` steps it, the
    // crate's own where it writes one (#162).
    if (by === "next" && drop === undefined && it.nth !== undefined) return it.nth(k);
    for (; k > 0; k--) {
      const at = it[by]();
      if (at.done) return at;
      drop?.(at.value);
    }
    return it[by]();
  };
  const stepped = new $Steps(
    () => {
      const at = skip(stepped.first ? 0 : step - 1, "next");
      stepped.first = false;
      return at;
    },
    () => $closed(it),
  );
  stepped.first = true;
  stepped.skip = skip;
  stepped.nth = (n) => {
    if (stepped.first) {
      stepped.first = false;
      const head = it.next();
      if (n === 0) return head;
      if (!head.done) drop?.(head.value);
      n -= 1;
    }
    return skip((n + 1) * step - 1, "next");
  };
  stepped.tryFold = (init, f, broke) => {
    let folded = init;
    if (stepped.first) {
      stepped.first = false;
      const head = it.next();
      if (head.done) return folded;
      folded = f(folded, head.value);
      if (broke(folded)) return folded;
    }
    for (let at = skip(step - 1, "next"); !at.done; at = skip(step - 1, "next")) {
      folded = f(folded, at.value);
      if (broke(folded)) return folded;
    }
    return folded;
  };
  stepped.fold = (init, f) => stepped.tryFold(init, f, () => false);
  return stepped;
}

// `it.step_by(step)` of a JS iterator that knows its back and its length,
// which knows its back too, as Rust's `StepBy` is double-ended where its
// source is exact-sized: `$stepBy`'s from the front; from the back, the
// last of those, found by the source's length, as std's `next_back_index`
// finds it.
function $stepByEnds(it, step) {
  const stepped = $stepBy(it, step);
  stepped.back = () => {
    const rem = it.len() % step;
    return stepped.skip(stepped.first ? (rem === 0 ? step - 1 : rem - 1) : rem, "back");
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
  const filter = new $Steps(kept(() => it.next()));
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
  const enumerated = new $Steps(
    () => {
      const step = enumerated.iter.next();
      return step.done ? step : pair(step, enumerated.count++);
    },
    () => $closed(enumerated.iter),
  );
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

// A flatten of a JS iterator that knows its back, `it.flatMap(each)`, each
// of whose items `each` makes what knows its back too, an array's `$iter`:
// which knows its own, as Rust's `Flatten` and `FlatMap` are double-ended
// where their source and its items are. From the front, the front inner's
// items, then the next of the source's, then, its source done, the back
// inner's; from the back, the same the other way, as std's `FlattenCompat`
// steps them, its source fused. Closing it closes its source.
function $flattenEnds(it, each) {
  const done = { done: true, value: undefined };
  const inner = { next: undefined, back: undefined };
  // Its source, once done from either end, stays done, as std's `Fuse` of
  // it does.
  let ended = false;
  const step = (by, own, other) => () => {
    for (;;) {
      if (inner[own] !== undefined) {
        const at = inner[own][by]();
        if (!at.done) return at;
        inner[own] = undefined;
      }
      const outer = ended ? done : it[by]();
      if (outer.done) {
        ended = true;
        if (inner[other] === undefined) return done;
        const at = inner[other][by]();
        if (at.done) inner[other] = undefined;
        return at;
      }
      inner[own] = $iter(each(outer.value));
    }
  };
  const flat = new $Steps(step("next", "next", "back"), () => $closed(it));
  flat.back = step("back", "back", "next");
  return flat;
}
