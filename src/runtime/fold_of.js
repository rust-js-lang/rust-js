// `it.fold(init, f)` of a JS iterator whose `fold` is std's adapter's over
// the crate's own, as a dictionary's `fold` is called, of the value it's of
// (#214).
function $foldOf(it, init, f) {
  return it.fold(init, f);
}

// `collect()` into a `Vec` of a `TrustedLen` iterator, as std's
// `extend_trusted` does it: each item through its `fold`, std's compiled one
// where it's std's struct, a zip's `SpecFold` say, which steps no item past
// its shorter side's (#380).
function $foldToArray(it) {
  return it.fold([], (items, x) => {
    items.push(x);
    return items;
  });
}

// `collect()` into a `Vec` of what may throw as it's stepped, a panic in a
// closure, of items with a destructor, `drop`, or from an owning source: as
// std's unwinds, what it collected dropped, then what's left of what it
// steps, closed (#422).
function $collectDropping(it, drop) {
  const items = [];
  try {
    for (let step = it.next(); !step.done; step = it.next()) items.push(step.value);
  } catch (error) {
    try {
      if (drop !== undefined) $dropItems(items, drop);
    } finally {
      it.return?.();
    }
    throw error;
  }
  // And at its end, as std's `collect` drops what it took then, what an
  // adapter that stops early left (ADR 0570).
  it.return?.();
  return items;
}
