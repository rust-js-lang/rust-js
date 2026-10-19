// An `f64`'s `midpoint(b)`, as std finds it: the sum halved where it can't
// go past the largest `f64`, else each halved first. std keeps a tiny one
// whole beside a huge one, which the huge one's half absorbs either way.
function $floatMidpoint(a, b) {
  const hi = Number.MAX_VALUE / 2;
  if (Math.abs(a) <= hi && Math.abs(b) <= hi) return (a + b) / 2;
  return a / 2 + b / 2;
}
