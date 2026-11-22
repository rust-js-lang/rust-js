// `a.wrapping_div(b)`, a number's or a BigInt's: `MIN / -1` is `MIN`, where
// it would overflow; a zero `b` panics as `/` does.
// Each division's panic raised where it was called, `raisedAt` (#2292).
function $wrappingDiv(a, b, raisedAt, min) {
  if (b == 0) throw $panic("attempt to divide by zero", raisedAt);
  if (a === min && b == -1) return a;
  return typeof a === "bigint" ? a / b : Math.trunc(a / b) + 0;
}

// `a.overflowing_div(b)`: what `wrapping_div` gives, and whether it wrapped,
// `MIN / -1`'s.
function $overflowingDiv(a, b, raisedAt, min) {
  return [$wrappingDiv(a, b, raisedAt, min), a === min && b == -1];
}

// `a.saturating_div(b)`: `MIN / -1` is `MAX`, where it would overflow.
function $saturatingDiv(a, b, raisedAt, min, max) {
  return a === min && b == -1 ? max : $wrappingDiv(a, b, raisedAt, min);
}

// `a.is_multiple_of(b)` of an unsigned number or BigInt: whether `b`
// divides it, of a zero `b` whether it's zero, as std's is.
function $isMultipleOf(a, b) {
  return b == 0 ? a == 0 : a % b == 0;
}

// `a.checked_next_multiple_of(b)` of an unsigned number or BigInt: the
// least multiple of `b` at least `a`, none of a zero `b` or past `max`.
function $checkedNextMultipleOf(a, b, max) {
  if (b == 0) return undefined;
  const r = a % b;
  const next = r == 0 ? a : a + (b - r);
  return next > max ? undefined : next;
}

// `a.next_multiple_of(b)` of an unsigned number or BigInt: the least
// multiple of `b` at least `a`, wrapped past `max` as overflow wraps; a
// zero `b` panics as `%` does.
function $nextMultipleOf(a, b, max, type) {
  if (b == 0) throw $panic("attempt to calculate the remainder with a divisor of zero", $stdSites.nextMultipleOf[type][0]);
  const r = a % b;
  const next = r == 0 ? a : a + (b - r);
  return next > max ? next - max - (typeof next === "bigint" ? 1n : 1) : next;
}
