// `s.get(start..end)`: the string between those bytes, or `undefined`,
// `None`, where `&s[start..end]` would panic: past the end, backwards, or
// inside a `char`.
function $strGet(s, start, end) {
  const length = $byteLen(s);
  end ??= length;
  if (start > end || end > length) return undefined;
  const from = $charBoundary(s, start);
  const to = $charBoundary(s, end);
  return from === undefined || to === undefined ? undefined : s.slice(from, to);
}
