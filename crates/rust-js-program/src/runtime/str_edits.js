// A `String`'s edits of a byte range (ADR 0323). `$strRange`: the UTF-16
// units of the bytes `start..end` of `s`, checked as std checks them,
// `slice::range`'s, then each a `char`'s boundary: `replace_range`'s message,
// or the other edits' assertion.
function $strRange(s, start, end, replacing, raisedAt, inclusive) {
  const length = $byteLen(s);
  end ??= length;
  $checkRange(start, end, length, raisedAt, inclusive);
  const from = $charBoundary(s, start);
  if (from === undefined) {
    throw $panic(replacing ? "start of range should be a character boundary" : "assertion failed: self.is_char_boundary(start)", raisedAt);
  }
  const to = $charBoundary(s, end);
  if (to === undefined) {
    throw $panic(replacing ? "end of range should be a character boundary" : "assertion failed: self.is_char_boundary(end)", raisedAt);
  }
  return [from, to];
}

// `s.split_off(at)`: what's kept, and the rest from the byte `at`.
function $strSplitOff(s, at, raisedAt) {
  const unit = $charBoundary(s, at);
  if (unit === undefined) throw $panic("assertion failed: self.is_char_boundary(at)", raisedAt);
  return [s.slice(0, unit), s.slice(unit)];
}

function $replaceRange(s, start, end, replacement, raisedAt, inclusive) {
  const [from, to] = $strRange(s, start, end, true, raisedAt, inclusive);
  return s.slice(0, from) + replacement + s.slice(to);
}

// `s.drain(range)`: what's kept, and the `char`s taken out.
function $strDrain(s, start, end, raisedAt, inclusive) {
  const [from, to] = $strRange(s, start, end, false, raisedAt, inclusive);
  return [s.slice(0, from) + s.slice(to), Array.from(s.slice(from, to))];
}

function $strExtendWithin(s, start, end, raisedAt, inclusive) {
  const [from, to] = $strRange(s, start, end, false, raisedAt, inclusive);
  return s + s.slice(from, to);
}

// `s.retain(keep)`, of a `keep` that may panic: each character it keeps, in
// order. Where it throws, a panic, past the first it didn't keep, `set` is
// given those it kept of the characters before, as std's `String::retain`
// leaves only them as it unwinds, what it hadn't asked of gone too (#1171);
// before that, `s` is left as it was, as 1.100's asks each in place till
// one goes (ADR 0109, amended).
function $retainText(s, keep, set) {
  let kept = "";
  let removed = false;
  for (const c of s) {
    if ($unwinds(() => keep(c), () => removed && set(kept))) kept += c;
    else removed = true;
  }
  return kept;
}
