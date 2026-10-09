// A `String`'s edits of a byte range (ADR 0323). `$strRange`: the UTF-16
// units of the bytes `start..end` of `s`, checked as std checks them,
// `slice::range`'s, then each a `char`'s boundary: `replace_range`'s message,
// or the other edits' assertion.
function $strRange(s, start, end, replacing) {
  const length = $byteLen(s);
  end ??= length;
  if (start > end) throw new Error(`slice index starts at ${start} but ends at ${end}`);
  if (end > length) throw new Error(`range end index ${end} out of range for slice of length ${length}`);
  const from = $charBoundary(s, start);
  if (from === undefined) {
    throw new Error(replacing ? "start of range should be a character boundary" : "assertion failed: self.is_char_boundary(start)");
  }
  const to = $charBoundary(s, end);
  if (to === undefined) {
    throw new Error(replacing ? "end of range should be a character boundary" : "assertion failed: self.is_char_boundary(end)");
  }
  return [from, to];
}

// `s.split_off(at)`: what's kept, and the rest from the byte `at`.
function $strSplitOff(s, at) {
  const unit = $charBoundary(s, at);
  if (unit === undefined) throw new Error("assertion failed: self.is_char_boundary(at)");
  return [s.slice(0, unit), s.slice(unit)];
}

function $replaceRange(s, start, end, replacement) {
  const [from, to] = $strRange(s, start, end, true);
  return s.slice(0, from) + replacement + s.slice(to);
}

// `s.drain(range)`: what's kept, and the `char`s taken out.
function $strDrain(s, start, end) {
  const [from, to] = $strRange(s, start, end, false);
  return [s.slice(0, from) + s.slice(to), Array.from(s.slice(from, to))];
}

function $strExtendWithin(s, start, end) {
  const [from, to] = $strRange(s, start, end, false);
  return s + s.slice(from, to);
}
