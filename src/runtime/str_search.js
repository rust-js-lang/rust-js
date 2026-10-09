// A `str`'s splits and searches from the end, by UTF-8 bytes (ADR 0323).

// `s.split_inclusive(p)`: each piece with the match that ends it, and none
// empty at the end.
function $splitInclusive(s, p) {
  const parts = [];
  let start = 0;
  if (p === "") {
    let unit = 0;
    for (const c of s) {
      parts.push(s.slice(start, unit));
      start = unit;
      unit += c.length;
    }
    parts.push(s.slice(start, unit));
    return parts;
  }
  for (let at = s.indexOf(p); at >= 0; at = s.indexOf(p, at + p.length)) {
    parts.push(s.slice(start, at + p.length));
    start = at + p.length;
  }
  if (start !== s.length) parts.push(s.slice(start));
  return parts;
}

// `s.rsplit_terminator(p)`: `rsplit`'s pieces, without the empty one the
// end leaves.
function $rsplitTerminator(s, p) {
  const parts = $rsplit(s, p);
  if (parts[0] === "") parts.shift();
  return parts;
}

// `s.rmatch_indices(p)`: each match from the end, none overlapping, with
// where it starts in UTF-8 bytes.
function $rmatchIndices(s, p) {
  if (p === "") return $matchIndices(s, p).reverse();
  const found = [];
  for (let at = s.lastIndexOf(p); at >= 0; at = at - p.length < 0 ? -1 : s.lastIndexOf(p, at - p.length)) {
    found.push([$byteLen(s.slice(0, at)), p]);
  }
  return found;
}

function $rmatches(s, p) {
  return $rmatchIndices(s, p).map(([, found]) => found);
}

// `s.split_at_checked(at)`: before and after the byte `at`, or `None` past
// the end or inside a `char`.
function $splitAtChecked(s, at) {
  const unit = $charBoundary(s, at);
  return unit === undefined ? undefined : [s.slice(0, unit), s.slice(unit)];
}

function $encodeUtf16(s) {
  return Array.from({ length: s.length }, (_, i) => s.charCodeAt(i));
}

// `String::from_utf16(units)`: `Ok` of its text, or `Err` of a lone
// surrogate; `from_utf16_lossy`, each of those U+FFFD.
function $fromUtf16(units) {
  const text = $utf16Text(units);
  return text.isWellFormed() ? { TAG: "Ok", _0: text } : { TAG: "Err", _0: undefined };
}

function $fromUtf16Lossy(units) {
  return $utf16Text(units).toWellFormed();
}

function $utf16Text(units) {
  let text = "";
  for (let i = 0; i < units.length; i += 4096) text += String.fromCharCode(...units.slice(i, i + 4096));
  return text;
}

// `s.floor_char_boundary(at)`: the `char` boundary at the byte `at` or before
// it; `ceil_char_boundary`, at or after it; the end past it.
function $floorCharBoundary(s, at) {
  let bytes = 0;
  for (const c of s) {
    const next = bytes + $byteLen(c);
    if (next > at) return bytes;
    bytes = next;
  }
  return bytes;
}

function $ceilCharBoundary(s, at) {
  let bytes = 0;
  for (const c of s) {
    if (bytes >= at) return bytes;
    bytes += $byteLen(c);
  }
  return bytes;
}
