// `(start..end).collect()`, a range's numbers in an array, room for them
// of `size` bytes each asked first, where it may not fit, as std's
// `collect` asks it of an iterator that knows its length (ADR 0774).
function $range(start, end, size) {
  const length = Math.max(0, end - start);
  if (size !== undefined) $roomFor(length, size);
  return Array.from({ length }, (_, i) => start + i);
}
