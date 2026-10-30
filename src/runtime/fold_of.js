// `it.fold(init, f)` of a JS iterator whose `fold` is std's adapter's over
// the crate's own, as a dictionary's `fold` is called, of the value it's of
// (#214); given `unwind`, what a stage's panic strands, `$foldDropping`'s,
// its own fold's to pass on (#423).
function $foldOf(it, init, f, unwind) {
  return it.fold(init, f, unwind);
}

// `it.fold(init, f)` of an accumulator with a destructor that a stage's
// panic may strand, `unwind` its drop: its own `fold`, given it, which
// passes it on to each stage; else stepped by `next`, where a step's throw
// drops what's folded, then what's left is closed, as std's adapter's fold
// closure drops the accumulator it holds, then its fold what it took
// (#423). Of an array, or given no `unwind`, its `reduce`.
function $foldDropping(it, init, f, unwind) {
  if (unwind === undefined || Array.isArray(it)) return it.reduce(f, init);
  if (typeof it.fold === "function") return it.fold(init, f, unwind);
  let folded = init;
  for (;;) {
    let step;
    try {
      step = it.next();
    } catch (error) {
      try {
        unwind(folded);
      } finally {
        it.return?.();
      }
      throw error;
    }
    if (step.done) return folded;
    try {
      folded = f(folded, step.value);
    } catch (error) {
      it.return?.();
      throw error;
    }
  }
}

// `collect()` into a `Vec` of a `TrustedLen` iterator, as std's
// `extend_trusted` does it: each item through its `fold`, std's compiled one
// where it's std's struct, a zip's `SpecFold` say, which steps no item past
// its shorter side's (#380).
//
// Where its fold throws, a panic in a closure, what it collected dropped
// by `drop`, of items with a destructor, and what's left of it closed, as
// std's unwinds, what's left first, `restFirst`, where it doesn't collect
// in place (#423). In place, what it collected is the fold's accumulator,
// std's `InPlaceDrop`, which the stage that panics drops before its fold
// drops what's left: the fold's `unwind`.
function $foldToArray(it, drop, restFirst = false) {
  const items = [];
  let unwound = false;
  const unwind =
    restFirst || drop === undefined
      ? undefined
      : () => {
          unwound = true;
          $dropItems(items, drop);
        };
  try {
    return it.fold(
      items,
      (items, x) => {
        items.push(x);
        return items;
      },
      unwind,
    );
  } catch (error) {
    $collectUnwound(it, items, unwound ? undefined : drop, restFirst);
    throw error;
  }
}

// `collect()` into a `Vec` of what may throw as it's stepped, a panic in a
// closure, of items with a destructor, `drop`, or from an owning source: as
// std's unwinds, what it collected dropped, then what's left of what it
// steps, closed (#422); or, `restFirst`, what's left first, where std
// collects it otherwise than in place, the fold's own dropped before the
// `Vec` (#423).
function $collectDropping(it, drop, restFirst = false) {
  const items = [];
  try {
    for (let step = it.next(); !step.done; step = it.next()) items.push(step.value);
  } catch (error) {
    $collectUnwound(it, items, drop, restFirst);
    throw error;
  }
  // And at its end, as std's `collect` drops what it took then, what an
  // adapter that stops early left (ADR 0570).
  it.return?.();
  return items;
}

// What a collect a panic stopped holds, dropped as std's unwinds: `items`,
// what it collected, by `drop`, and what's left of `it`, closed; what's
// left first, `restFirst`, where std's fold drops it before its `Vec`
// (#422, #423).
function $collectUnwound(it, items, drop, restFirst) {
  const collected = () => {
    if (drop !== undefined) $dropItems(items, drop);
  };
  const rest = () => it.return?.();
  const [first, then] = restFirst ? [rest, collected] : [collected, rest];
  try {
    first();
  } finally {
    then();
  }
}
