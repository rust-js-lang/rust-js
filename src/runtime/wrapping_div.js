// `a.wrapping_div(b)`, a number's or a BigInt's: `MIN / -1` is `MIN`, where
// it would overflow; a zero `b` panics as `/` does.
function $wrappingDiv(a, b, min) {
  if (b == 0) throw new Error("attempt to divide by zero");
  if (a === min && b == -1) return a;
  return typeof a === "bigint" ? a / b : Math.trunc(a / b) + 0;
}
