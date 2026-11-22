// `s.pop()`: `s` without its last `char`, and that `char`, or `undefined`,
// `None`, if it's empty.
function $strPop(s) {
  if (s === "") return [s, undefined];
  const low = s.charCodeAt(s.length - 1);
  const high = s.length > 1 ? s.charCodeAt(s.length - 2) : 0;
  const size = low >= 0xdc00 && low <= 0xdfff && high >= 0xd800 && high <= 0xdbff ? 2 : 1;
  return [s.slice(0, s.length - size), s.slice(s.length - size)];
}
