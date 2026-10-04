// `s.trim_matches(p)`, or `trim_start_matches` and `trim_end_matches`: `p`
// taken off as often as it matches there, a `char` or a string, or each
// `char` a closure says.
function $trimMatches(s, p, start = true, end = true) {
  let from = 0;
  let to = s.length;
  const chars = typeof p === "function" ? Array.from(s) : undefined;
  if (start) {
    if (chars) {
      for (const c of chars) {
        if (!p(c)) break;
        from += c.length;
      }
    } else if (p !== "") {
      while (s.startsWith(p, from)) from += p.length;
    }
  }
  if (end) {
    if (chars) {
      for (let i = chars.length - 1; i >= 0 && to > from && p(chars[i]); i--) to -= chars[i].length;
    } else if (p !== "") {
      while (to - p.length >= from && s.startsWith(p, to - p.length)) to -= p.length;
    }
  }
  return s.slice(from, to);
}
