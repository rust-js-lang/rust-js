// `a.checked_rem(b)`, a number's or a BigInt's: `None` of a zero `b`, or of
// `MIN % -1`, which overflows; never JS's `-0`.
function $checkedRem(a, b, min) {
  if (b == 0 || (a === min && b == -1)) return undefined;
  const rem = a % b;
  return typeof rem === "bigint" ? rem : rem + 0;
}
