// `{:e}` of a number, as Rust writes it: the shortest digits that read back
// as `x`, `1.2345e3`. An `f32`'s are its own (ADR 0122), not its `f64`'s; a
// 64-bit integer, a BigInt, has every digit; a negative zero keeps its sign.
// Given a precision, `{:.2e}`, that many digits after the first, of `x`'s
// exact value rounded half to even, as Rust rounds a float's and an
// integer's.
function $lowerExp(x, f32, precision) {
  if (precision !== undefined) return $lowerExpRounded(x, precision);
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
function $lowerExpRounded(x, precision) {
  if (typeof x === "number" && Number.isNaN(x)) return "NaN";
  if (typeof x === "number" && !Number.isFinite(x)) return x > 0 ? "inf" : "-inf";
  const negative = typeof x === "bigint" ? x < 0n : x < 0 || Object.is(x, -0);
  // |x| as an exact fraction, `numerator / denominator`.
  let numerator;
  let denominator = 1n;
  if (typeof x === "bigint") {
    numerator = negative ? -x : x;
  } else {
    const view = new DataView(new ArrayBuffer(8));
    view.setFloat64(0, Math.abs(x));
    const bits = view.getBigUint64(0);
    const exponent = Number((bits >> 52n) & 2047n);
    const fraction = bits & ((1n << 52n) - 1n);
    numerator = exponent === 0 ? fraction : fraction | (1n << 52n);
    const shift = exponent === 0 ? -1074 : exponent - 1075;
    if (shift >= 0) numerator <<= BigInt(shift);
    else denominator <<= BigInt(-shift);
  }
  const sign = negative ? "-" : "";
  const zeros = "0".repeat(precision);
  if (numerator === 0n) return sign + "0" + (precision > 0 ? "." + zeros : "") + "e0";
  // Its decimal exponent: 10^e <= |x| < 10^(e+1). Its digits' lengths'
  // difference is it or one more.
  let e = numerator.toString().length - denominator.toString().length;
  const pow = (n) => 10n ** BigInt(n);
  const below = (k) => (k >= 0 ? numerator < denominator * pow(k) : numerator * pow(-k) < denominator);
  if (below(e)) e--;
  // Its digits, `precision` after the first, rounded half to even.
  const scale = precision - e;
  const top = scale >= 0 ? numerator * pow(scale) : numerator;
  const bottom = scale >= 0 ? denominator : denominator * pow(-scale);
  let digits = top / bottom;
  const twice = 2n * (top % bottom);
  if (twice > bottom || (twice === bottom && digits % 2n === 1n)) digits += 1n;
  // A carry, `9.96` to `10.0`: one digit more, the exponent one up.
  if (digits === pow(precision + 1)) {
    digits /= 10n;
    e++;
  }
  const text = digits.toString();
  return sign + text[0] + (precision > 0 ? "." + text.slice(1) : "") + "e" + e;
}
