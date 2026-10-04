// `s.starts_with(p)`, or `ends_with`, of a predicate of a `char`: whether it
// holds for the first, or last, `char`, of which an empty string has none.
function $startsBy(s, p, end) {
  if (s === "") return false;
  const at = end ? s.length - (/[\udc00-\udfff]$/.test(s) && s.length > 1 ? 2 : 1) : 0;
  return p(String.fromCodePoint(s.codePointAt(at)));
}
