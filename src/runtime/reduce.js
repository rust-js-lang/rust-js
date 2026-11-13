// `it.reduce(f)`: its first item, folded with each after it by `f`, or
// `None` of none, where JS's `reduce` throws, closed, as std's `next()?`
// drops it (#242); boxed where it may look like `None` (ADRs 0051, 0479).
// Given `unwind`, what drops what it folded where a stage throws, a panic,
// passed to its fold (#423); and closed where its first step throws.
function $reduce(it, f, boxed = false, unwind = undefined) {
  const first = $unwinds(
    () => it.next(),
    () => it.return?.(),
  );
  if (first.done) {
    it.return?.();
    return undefined;
  }
  const value = $foldDropping(it, first.value, f, unwind);
  return boxed ? $some(value) : value;
}

// `max()` and `min()` of `items`, an array or an iterator, of a type JS's
// `<` doesn't order: std's `reduce(Ord::max)` since 1.100, `extreme` of
// each two, the one kept and the next, the type's own `max` or std's, which
// drops the one it doesn't give back (ADR 0109, amended). Of items with a
// destructor, `drop`: what's left of an array dropped where a step throws,
// and the one kept (#423).
function $reduceBy(items, extreme, boxed = false, drop = undefined) {
  const it = Array.isArray(items) && drop !== undefined ? $closing($iter(items), $dropEach(drop)) : Iterator.from(items);
  // Two arguments alone: a JS `reduce` gives its function the index too.
  // The one kept dropped only where a stage throws, inside the fold: once
  // it's folded it's given back, so a panic as what it stepped is closed
  // leaks it, as #1655's `max_by` does.
  return $reduce(it, (kept, item) => extreme(kept, item), boxed, drop);
}
