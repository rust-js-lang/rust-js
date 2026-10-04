// `{:e}` of a number, as Rust writes it: the shortest digits that read back
// as `x`, `1.2345e3`. An `f32`'s are its own (ADR 0122), not its `f64`'s; a
// 64-bit integer, a BigInt, has every digit; a negative zero keeps its sign.
function $lowerExp(x, f32) {
  if (typeof x === "bigint") {
    const digits = (x < 0n ? -x : x).toString();
    const kept = digits.replace(/0+$/, "") || "0";
    const rest = kept.length > 1 ? "." + kept.slice(1) : "";
    return (x < 0n ? "-" : "") + kept[0] + rest + "e" + (digits.length - 1);
  }
  if (Number.isNaN(x)) return "NaN";
  if (!Number.isFinite(x)) return x > 0 ? "inf" : "-inf";
  const sign = x < 0 || Object.is(x, -0) ? "-" : "";
  if (x === 0) return sign + "0e0";
  if (f32) {
    const [digits, point] = $f32Digits(Math.abs(x));
    const rest = digits.length > 1 ? "." + digits.slice(1) : "";
    return sign + digits[0] + rest + "e" + (point - 1);
  }
  return sign + Math.abs(x).toExponential().replace("e+", "e");
}
