// `s.split(p)`, or `s.splitn(n, p)`, of a predicate of a `char`: its
// pieces, of `splitn` at most `n`, the last what's left whole.
function $splitBy(s, matches, n = Infinity) {
  return Array.from($lazySplitBy(s, matches, n));
}
