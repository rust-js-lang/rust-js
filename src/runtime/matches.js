// `s.matches(p)`: each match, from the start, none overlapping.
function $matches(s, p) {
  if (p === "") return Array.from({ length: Array.from(s).length + 1 }, () => "");
  const found = [];
  for (let at = s.indexOf(p); at >= 0; at = s.indexOf(p, at + p.length)) found.push(p);
  return found;
}

// `s.matches(p)` of a predicate of a `char`: each `char` it holds of, from
// the start, or of `s.rmatches(p)`, `rev`, from the end, as Rust asks it.
function $matchesBy(s, matches, rev = false) {
  const chars = Array.from(s);
  if (rev) chars.reverse();
  return chars.filter((c) => matches(c));
}
