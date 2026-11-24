// `(start..end).collect()`, a range's numbers in an array, room for them
// of `size` bytes each asked first, where it may not fit, as std's
// `collect` asks it of an iterator that knows its length (ADR 0774). Of a
// length past `usize::MAX`, `0..=usize::MAX`'s, whose hint has no upper
// bound, the `capacity overflow` std's `TrustedLen` collect panics with at
// its own line, `vecFromTrustedLen`'s (#2488).
function $range(start, end, size) {
  const length = Math.max(0, end - start);
  if (size !== undefined && length > 2 ** 32 - 1) throw $panic("capacity overflow", $stdSites.vecFromTrustedLen[0]);
  if (size !== undefined) $roomFor(length, size);
  return Array.from({ length }, (_, i) => start + i);
}
