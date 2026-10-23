// A `str`'s splits and searches of a predicate of a `char`, a closure, a
// function or a set of `char`s (ADR 0157): each match is one `char`, where
// it starts in UTF-16 units, `unit`, and in UTF-8 bytes, `byte` (#118).
function $charMatches(s, test) {
  const found = [];
  let unit = 0;
  let byte = 0;
  for (const c of s) {
    if (test(c)) found.push({ unit, byte, c });
    unit += c.length;
    byte += $byteLen(c);
  }
  return found;
}

// `s.rsplit(p)`, or `s.rsplitn(n, p)`: its pieces from the end, of
// `rsplitn` at most `n`, the last what's left of it whole.
function $rsplitBy(s, test, n = Infinity) {
  if (n === 0) return [];
  const found = $charMatches(s, test);
  const pieces = [];
  let end = s.length;
  for (let k = found.length - 1; k >= 0 && pieces.length < n - 1; k--) {
    pieces.push(s.slice(found[k].unit + found[k].c.length, end));
    end = found[k].unit;
  }
  pieces.push(s.slice(0, end));
  return pieces;
}

// `s.split_terminator(p)`: `split`'s pieces without the empty one its end
// leaves; `rsplit_terminator`'s, `rev`, from the end.
function $splitTerminatorBy(s, test, rev = false) {
  const pieces = $splitBy(s, test);
  if (pieces[pieces.length - 1] === "") pieces.pop();
  return rev ? pieces.reverse() : pieces;
}

// `s.split_inclusive(p)`: each piece with the `char` that ends it, and none
// empty at the end.
function $splitInclusiveBy(s, test) {
  const pieces = [];
  let piece = "";
  for (const c of s) {
    piece += c;
    if (test(c)) {
      pieces.push(piece);
      piece = "";
    }
  }
  if (piece !== "") pieces.push(piece);
  return pieces;
}

// `s.match_indices(p)`: each `char` it holds of, and where it starts in
// UTF-8 bytes; `rmatch_indices`', `rev`, from the end.
function $matchIndicesBy(s, test, rev = false) {
  const found = $charMatches(s, test).map(({ byte, c }) => [byte, c]);
  return rev ? found.reverse() : found;
}

// `s.split_once(p)`: what's before its first match and after it, or
// `None`; `rsplit_once`'s, `last`, of its last.
function $splitOnceBy(s, test, last = false) {
  const found = $charMatches(s, test);
  const at = last ? found[found.length - 1] : found[0];
  return at === undefined ? undefined : [s.slice(0, at.unit), s.slice(at.unit + at.c.length)];
}

// `s.strip_prefix(p)`: what's after its first `char`, where `p` holds of it,
// or `None`; `strip_suffix`'s, `suffix`, before its last.
function $stripBy(s, test, suffix = false) {
  const chars = Array.from(s);
  const c = suffix ? chars[chars.length - 1] : chars[0];
  if (c === undefined || !test(c)) return undefined;
  return suffix ? s.slice(0, s.length - c.length) : s.slice(c.length);
}

// `s.replace(p, to)`: each `char` it holds of, `to`; `replacen`'s, the first
// `n` of them.
function $replaceBy(s, test, to, n = Infinity) {
  let out = "";
  let count = 0;
  for (const c of s) {
    if (count < n && test(c)) {
      out += to;
      count++;
    } else {
      out += c;
    }
  }
  return out;
}
