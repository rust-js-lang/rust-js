function $strSlice(s, start, end) {
  const length = $byteLen(s);
  end ??= length;
  if (start > length) throw new Error(`start byte index ${start} is out of bounds for string of length ${length}`);
  if (end > length) throw new Error(`end byte index ${end} is out of bounds for string of length ${length}`);
  if (start > end) throw new Error(`byte range starts at ${start} but ends at ${end}`);
  const from = $unitAt(s, start, "start");
  const part = s.slice(from, $unitAt(s, end, "end"));
  // A long part's facts are of where it is in `s` (#1184).
  if (part.length >= $textLong) $textPart(part, s, start, from, end - start);
  return part;
}

// The UTF-16 unit byte `at` of `s` starts at, `at` within it, or Rust's
// panic where it's inside a character.
function $unitAt(s, at, which) {
  const unit = $charBoundary(s, at);
  if (unit !== undefined) return unit;
  let from = at - 1;
  while ($charBoundary(s, from) === undefined) from--;
  const c = String.fromCodePoint(s.codePointAt($charBoundary(s, from)));
  throw new Error(`${which} byte index ${at} is not a char boundary; it is inside ${$debugStr(c, "'")} (bytes ${from}..${from + $byteLen(c)} of string)`);
}
