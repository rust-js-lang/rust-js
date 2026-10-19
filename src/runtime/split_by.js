// `s.split(p)`, or `s.splitn(n, p)`, of a predicate of a `char`: its
// pieces, of `splitn` at most `n`, the last what's left whole.
function $splitBy(s, matches, n = Infinity) {
  if (n === 0) return [];
  const pieces = [""];
  for (const c of s) {
    if (pieces.length < n && matches(c)) {
      pieces.push("");
    } else {
      pieces[pieces.length - 1] += c;
    }
  }
  return pieces;
}
