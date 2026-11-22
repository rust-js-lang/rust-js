function $windows(v, size) {
  if (size === 0) {
    throw $stringPanic("window size must be non-zero");
  }
  const parts = Array.from({ length: Math.max(0, v.length - size + 1) }, (_, i) => $part(v, i, i + size));
  // What std's `Debug` of what's left shows, `Windows { v, size }` (#2252).
  parts[$partsOf] = { source: v, size, windows: true };
  return parts;
}
