// What a double-ended iterator does from its back, of a JS iterator that
// knows its back, a `$iter` or a `$iterator` given its `nextBack`, as Rust's
// `next_back` steps it (ADR 0429).

// `it.nth_back(n)`: its `n`th item from the back, each before it stepped
// past, and dropped, `drop`, where items have a destructor, or `undefined`,
// `None`; of items that could look like `None`, its `Some`, `$some`, given
// as `found` (ADR 0051). Its own `nth_back`, where it reaches the crate's
// (#220).
function $nthBack(it, n, found = (item) => item, drop, slices = true) {
  if (it.nthBack !== undefined) {
    const step = it.nthBack(n);
    return step.done ? undefined : found(step.value);
  }
  // Of an array's items it owns, a `Vec`'s `IntoIter`'s, what it passes
  // over dropped as one slice, front to back, as std's `advance_back_by`
  // drops them; not of one that drops each, `slices` false (#234).
  if (slices && drop !== undefined && it.items !== undefined) {
    const k = Math.min(n, it.len());
    const passed = it.items.slice(it.end - k, it.end);
    it.end -= k;
    for (const item of passed) drop(item);
    if (k < n) return undefined;
    const step = it.back();
    return step.done ? undefined : found(step.value);
  }
  for (let step = it.back(); !step.done; step = it.back()) {
    if (n-- === 0) return found(step.value);
    drop?.(step.value);
  }
  return undefined;
}

// `it.rfold(init, f)`: `f` of each item from the back, given what it gave;
// its own `rfold`, where it reaches the crate's (#220). Consuming it, it
// closes it, as Rust's `rfold` drops what it takes at its end (#206).
function $foldBack(it, init, f) {
  if (it.rfold !== undefined) return it.rfold(init, f);
  let folded = init;
  for (let step = it.back(); !step.done; step = it.back()) folded = f(folded, step.value);
  it.return?.();
  return folded;
}

// `it.rfind(p)`: the first item from the back `p` holds of, or `undefined`;
// `found` and `drop`, of each it passes over, as `$nthBack`'s. Std's, its
// `try_rfold`, where that reaches the crate's own (#220).
function $findBack(it, p, found = (item) => item, drop) {
  if (it.tryRfold !== undefined) {
    let hit;
    const hits = it.tryRfold(
      false,
      (_, x) => {
        if (!p(x)) {
          drop?.(x);
          return false;
        }
        hit = x;
        return true;
      },
      (hits) => hits,
    );
    return hits ? found(hit) : undefined;
  }
  for (let step = it.back(); !step.done; step = it.back()) {
    if (p(step.value)) return found(step.value);
    drop?.(step.value);
  }
  return undefined;
}

// What an adapter stepped from its back asks of what it steps, as std's
// do (#220): its own `try_rfold`, where it reaches the crate's, else
// std's, of its `next_back`, stopped where `broke` holds of what it's made.
function $tryRfoldOf(it, init, f, broke) {
  if (it.tryRfold !== undefined) return it.tryRfold(init, f, broke);
  let folded = init;
  for (let step = it.back(); !step.done; step = it.back()) {
    folded = f(folded, step.value);
    if (broke(folded)) return folded;
  }
  return folded;
}

// And its `nth_back(k)`, a step: its own, where it reaches the crate's,
// else std's, `k` stepped past from its back, each dropped, `drop`, then
// the next.
function $nthBackOf(it, k, drop) {
  if (it.nthBack !== undefined) return it.nthBack(k);
  for (; k > 0; k--) {
    const step = it.back();
    if (step.done) return step;
    drop?.(step.value);
  }
  return it.back();
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
  // Std's `Map::rfold` and `try_rfold`, its source's of each mapped, the
  // crate's own where it reaches it (#220).
  if (it.rfold !== undefined) map.rfold = (init, g) => it.rfold(init, (a, x) => g(a, f(x)));
  if (it.tryRfold !== undefined) map.tryRfold = (init, g, broke) => it.tryRfold(init, (a, x) => g(a, f(x)), broke);
  map.from = [it];
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
  // Std's `Chain::rfold` and `try_rfold`, `b`'s then `a`'s, and its
  // `nth_back`, `b` stepped back past `n`, then `a`'s own `nth_back`, where
  // they reach the crate's own (#220).
  if (a.rfold !== undefined || b.rfold !== undefined) {
    chain.rfold = (init, f) => {
      let folded = init;
      if (sides.b !== undefined) folded = $foldBack(sides.b, folded, f);
      if (sides.a !== undefined) folded = $foldBack(sides.a, folded, f);
      return folded;
    };
  }
  if (a.tryRfold !== undefined || b.tryRfold !== undefined) {
    chain.tryRfold = (init, f, broke) => {
      let folded = init;
      if (sides.b !== undefined) {
        folded = $tryRfoldOf(sides.b, folded, f, broke);
        if (broke(folded)) return folded;
        sides.b = undefined;
      }
      return sides.a === undefined ? folded : $tryRfoldOf(sides.a, folded, f, broke);
    };
  }
  if (a.nthBack !== undefined) {
    chain.nthBack = (n) => {
      if (sides.b !== undefined) {
        for (; n > 0; n--) if (sides.b.back().done) break;
        if (n === 0) {
          const at = sides.b.back();
          if (!at.done) return at;
        }
        sides.b = undefined;
      }
      return sides.a === undefined ? done : sides.a.nthBack(n);
    };
  }
  chain.from = [a, b];
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
    // crate's own where it writes one (#162); from the back, its `nth_back`,
    // as std's `next_back` asks it (#245).
    if (by === "next" && (drop === undefined || it.ownNth) && it.nth !== undefined) return it.nth(k);
    if (by === "back" && drop === undefined && it.nthBack !== undefined) return it.nthBack(k);
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
  // Its `nth` its source's, which drops what it passes over itself, a
  // reversed `Vec`'s as one slice (#234).
  if (it.ownNth) stepped.ownNth = true;
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
  // Its `fold`, its `try_fold` never stopped, then closed, as `fold(self)`
  // drops what it has left at its end (#242).
  stepped.fold = (init, f) => {
    const folded = stepped.tryFold(init, f, () => false);
    stepped.return();
    return folded;
  };
  stepped.from = [it];
  return stepped;
}

// `it.step_by(step)` of a JS iterator that knows its back and its length,
// which knows its back too, as Rust's `StepBy` is double-ended where its
// source is exact-sized: `$stepBy`'s from the front; from the back, the
// last of those, found by the source's length, as std's `next_back_index`
// finds it.
function $stepByEnds(it, step) {
  const stepped = $stepBy(it, step);
  // Std's `next_back_index`: how many its back passes over first.
  const backIndex = () => {
    const rem = it.len() % step;
    return stepped.first ? (rem === 0 ? step - 1 : rem - 1) : rem;
  };
  stepped.back = () => stepped.skip(backIndex(), "back");
  // Std's `nth_back(n)`: its source's, past `n` steps and its back's index,
  // one call, the crate's own where it writes one (#245).
  stepped.nthBack = (n) => stepped.skip(n * step + backIndex(), "back");
  return stepped;
}

// A filter of a JS iterator that knows its back, `it.filter(p)`, which
// knows it too, as Rust's `Filter` is double-ended where its source is:
// each of its source's items `p` keeps, from either end, `p` asked of each
// as it's reached (ADR 0526), and each it doesn't dropped by `drop`, of
// items with a destructor (ADR 0871). Closing it closes its source (ADR 0465, #225).
function $filterEnds(it, p, drop) {
  const kept = (step) => () => {
    for (let at = step(); !at.done; at = step()) {
      if (p(at.value)) return at;
      drop?.(at.value);
    }
    return { done: true, value: undefined };
  };
  const filter = new $Steps(
    kept(() => it.next()),
    () => $closed(it),
  );
  if (it.back !== undefined) filter.back = kept(() => it.back());
  // Std's `Filter::next_back`, its source's `rfind`, a `try_rfold`, where
  // that reaches the crate's own (#220).
  if (it.tryRfold !== undefined) {
    filter.back = () => {
      let hit;
      const hits = it.tryRfold(false, (_, x) => (p(x) ? ((hit = x), true) : (drop?.(x), false)), (hits) => hits);
      return hits ? { done: false, value: hit } : { done: true, value: undefined };
    };
  }
  // Std's `Filter::rfold` and `try_rfold`, its source's of each kept (#220).
  if (it.rfold !== undefined) filter.rfold = (init, f) => it.rfold(init, (a, x) => (p(x) ? f(a, x) : (drop?.(x), a)));
  if (it.tryRfold !== undefined) {
    filter.tryRfold = (init, f, broke) => it.tryRfold(init, (a, x) => (p(x) ? f(a, x) : (drop?.(x), a)), broke);
  }
  filter.from = [it];
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
  // Std's `fold` and `try_fold`: its source's, each item counted.
  if (iter.fold !== undefined) {
    enumerated.fold = (init, f) => enumerated.iter.fold(init, (a, x) => f(a, [enumerated.count++, x]));
  }
  enumerated.tryFold = (init, f, broke) =>
    $lent(enumerated.iter).tryFold(init, (a, x) => f(a, [enumerated.count++, x]), broke);
  // Std's `rfold`, `try_rfold` and `nth_back`: its source's, each counted
  // from where it is, the crate's own where they reach it (#220).
  if (iter.rfold !== undefined) {
    enumerated.rfold = (init, f) => {
      let at = enumerated.count + enumerated.iter.len();
      return enumerated.iter.rfold(init, (a, x) => f(a, [--at, x]));
    };
  }
  if (iter.tryRfold !== undefined) {
    enumerated.tryRfold = (init, f, broke) => {
      let at = enumerated.count + enumerated.iter.len();
      return enumerated.iter.tryRfold(init, (a, x) => f(a, [--at, x]), broke);
    };
  }
  if (iter.nthBack !== undefined) {
    enumerated.nthBack = (k) => {
      const step = enumerated.iter.nthBack(k);
      return pair(step, enumerated.count + enumerated.iter.len());
    };
  }
  // Std's `nth(k)`: its source's, counted `k` more, the crate's own where
  // it writes one (#182); its own where its source's is, which drops what
  // it passes itself, a reversed `Vec`'s, as a `step_by` over it asks (#277).
  if (iter.ownNth) enumerated.ownNth = true;
  if (iter.nth !== undefined) {
    enumerated.nth = (k) => {
      const step = enumerated.iter.nth(k);
      if (step.done) return step;
      const at = enumerated.count + k;
      enumerated.count = at + 1;
      return { done: false, value: [at, step.value] };
    };
  }
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
  flat.from = [it];
  return flat;
}
