// `s.match_indices(p)`: each match, from the start, none overlapping, with
// where it starts in UTF-8 bytes. An empty pattern matches at each `char`'s
// ends.
function $matchIndices(s, p) {
  const found = [];
  let bytes = 0;
  if (p === "") {
    for (const c of s) {
      found.push([bytes, ""]);
      bytes += $byteLen(c);
    }
    found.push([bytes, ""]);
    return found;
  }
  let unit = 0;
  for (let at = s.indexOf(p); at >= 0; at = s.indexOf(p, at + p.length)) {
    bytes += $byteLen(s.slice(unit, at));
    unit = at;
    found.push([bytes, p]);
  }
  return found;
}
