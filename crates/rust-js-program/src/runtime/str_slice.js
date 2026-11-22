function $strSlice(s, start, raisedAt, end, inclusive) {
  const length = $byteLen(s);
  end ??= length;
  // Of `a..=b`, `end` is `b + 1`: past the string's end, core checks `b`
  // itself, then `a`'s boundary, and says `b` (#1935).
  const past = inclusive && end > length;
  if (past) end -= 1;
  if (start > length) throw $stringPanic(`start byte index ${start} is out of bounds for string of length ${length}`, raisedAt);
  if (end > length) throw $stringPanic(`end byte index ${end} is out of bounds for string of length ${length}`, raisedAt);
  if (start > end) throw $stringPanic(`byte range starts at ${start} but ends at ${end}`, raisedAt);
  const from = $unitAt(s, start, "start", raisedAt);
  if (past) throw $stringPanic(`end byte index ${end} is out of bounds for string of length ${length}`, raisedAt);
  const part = s.slice(from, $unitAt(s, end, "end", raisedAt));
  // A long part's facts are of where it is in `s` (#1184).
  if (part.length >= $textLong) $textPart(part, s, start, from, end - start);
  return part;
}

// The UTF-16 unit byte `at` of `s` starts at, `at` within it, or Rust's
// panic where it's inside a character.
function $unitAt(s, at, which, raisedAt) {
  const unit = $charBoundary(s, at);
  if (unit !== undefined) return unit;
  let from = at - 1;
  while ($charBoundary(s, from) === undefined) from--;
  const c = String.fromCodePoint(s.codePointAt($charBoundary(s, from)));
  throw $stringPanic(`${which} byte index ${at} is not a char boundary; it is inside ${$debugStr(c, "'")} (bytes ${from}..${from + $byteLen(c)} of string)`, raisedAt);
}
