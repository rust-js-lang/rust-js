// `s.rsplit(p)`, and `s.rsplitn(n, p)`: its pieces from the end, searched
// from the end as Rust's are, so overlapping matches split where Rust's do,
// `"aaa".rsplit("aa")` being `["", "a"]`. An empty pattern matches at each
// `char`'s ends, `"ab".rsplit("")` being `["", "b", "a", ""]`.
function $rsplit(s, p, n = Infinity) {
  if (n === 0) return [];
  const parts = [];
  // Where the piece being cut ends, and where a match may start, at most.
  let end = s.length;
  let search = s.length;
  while (parts.length < n - 1) {
    let at;
    if (p === "") {
      if (search < 0) break;
      at = search;
      const low = s.charCodeAt(search - 1);
      search -= search > 1 && low >= 0xdc00 && low <= 0xdfff ? 2 : 1;
    } else {
      at = search - p.length < 0 ? -1 : s.lastIndexOf(p, search - p.length);
      if (at < 0) break;
      search = at;
    }
    parts.push(s.slice(at + p.length, end));
    end = at;
  }
  parts.push(s.slice(0, end));
  return parts;
}
