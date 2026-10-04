// `s.matches(p)`: each match, from the start, none overlapping.
function $matches(s, p) {
  if (p === "") return Array.from({ length: Array.from(s).length + 1 }, () => "");
  const found = [];
  for (let at = s.indexOf(p); at >= 0; at = s.indexOf(p, at + p.length)) found.push(p);
  return found;
}
