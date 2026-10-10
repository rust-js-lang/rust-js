// `s.strip_circumfix(prefix, suffix)`: `strip_prefix`'s, then
// `strip_suffix`'s of what's left, or `undefined`, `None`.
function $stripCircumfix(s, prefix, suffix) {
  const rest = $stripPrefix(s, prefix);
  return rest === undefined ? undefined : $stripSuffix(rest, suffix);
}
