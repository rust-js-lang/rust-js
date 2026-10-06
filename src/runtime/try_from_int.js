function $tryFromInt(x, lo, hi) {
  if (x < lo) return { TAG: "Err", _0: "number too small to fit in target type" };
  if (x > hi) return { TAG: "Err", _0: "number too large to fit in target type" };
  return { TAG: "Ok", _0: typeof hi === "bigint" ? BigInt(x) : Number(x) };
}
