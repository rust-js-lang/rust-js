// `s.remove(at)`: `s` without the `char` at the byte `at`, and that `char`,
// panicking as Rust's does at the end, past it or inside a `char`.
function $strRemove(s, at, raisedAt) {
  if (at === $byteLen(s)) throw $panic("cannot remove a char from the end of a string", raisedAt);
  const rest = $strSlice(s, at);
  const c = String.fromCodePoint(rest.codePointAt(0));
  return [s.slice(0, s.length - rest.length) + rest.slice(c.length), c];
}
