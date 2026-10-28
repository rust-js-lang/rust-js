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
