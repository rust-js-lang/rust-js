// `s.find(p)`, or `rfind`, of a predicate of a `char`, a closure's, a
// function's or a set's: the byte where the first, or last, `char` it holds
// for starts, or `undefined`.
function $findBy(s, p, last) {
  let bytes = 0;
  let found;
  for (const c of s) {
    if (p(c)) {
      found = bytes;
      if (!last) return found;
    }
    bytes += $byteLen(c);
  }
  return found;
}
