// A float's `fmt` into a writer, a piece at a time, as core's `flt2dec`
// makes its parts and `Formatter::pad_formatted_parts` writes them (#1652,
// ADR 1135): its sign, each part, its zeros by at most 64, and the padding
// to its width, each fill char a `write_char`. `trait` is the one it's shown
// by, `"D"` `Display`, `"G"` `Debug`, `"e"` `LowerExp`, `"E"` `UpperExp`;
// `f32` whether it's an `f32`, whose digits are its own.
function $writeFloat(f, x, trait, f32, options = {}) {
  const [sign, parts] = $floatParts(x, trait, f32, options);
  const partsLength = parts.reduce((n, part) => n + (typeof part === "number" ? part : part.length), 0);
  if (options.width === undefined) return $writeFloatParts(f, sign, parts);
  let width = options.width;
  let fill = options.fill ?? " ";
  let align = options.align ?? "Right";
  let shownSign = sign;
  if (options.zero) {
    // A sign always goes first, written even where it's none, `""`.
    f.write(sign);
    width = Math.max(0, width - sign.length);
    shownSign = "";
    fill = "0";
    align = "Right";
  }
  const length = shownSign.length + partsLength;
  if (width <= length) return $writeFloatParts(f, shownSign, parts);
  const after = $writePadding(f, width - length, align, fill);
  $writeFloatParts(f, shownSign, parts);
  for (let i = 0; i < after; i++) f.writeChar(fill);
}
// `write_formatted_parts`: the sign where there's one, then each part, a
// run of zeros, a number, written as a number of zeros, by at most 64.
function $writeFloatParts(f, sign, parts) {
  if (sign !== "") f.write(sign);
  for (const part of parts) {
    if (typeof part !== "number") {
      f.write(part);
      continue;
    }
    let zeros = part;
    while (zeros > 64) {
      f.write("0".repeat(64));
      zeros -= 64;
    }
    if (zeros > 0) f.write("0".repeat(zeros));
  }
}
// `flt2dec`'s sign and parts of `x` shown by `trait`: each a string, or a
// number of zeros (`Part::Zero`).
function $floatParts(x, trait, f32, options) {
  const negative = !Number.isNaN(x) && (x < 0 || Object.is(x, -0));
  const sign = Number.isNaN(x) ? "" : negative ? "-" : options.plus ? "+" : "";
  if (Number.isNaN(x)) return [sign, ["NaN"]];
  if (!Number.isFinite(x)) return [sign, ["inf"]];
  const size = Math.abs(x);
  const precision = options.precision;
  const upper = trait === "E";
  if (trait === "e" || trait === "E") {
    if (precision === undefined) {
      if (size === 0) return [sign, [upper ? "0E0" : "0e0"]];
      const [digits, point] = $floatShortest(size, f32);
      return [sign, $floatExpParts(digits, point, 0, upper)];
    }
    const count = precision + 1;
    if (size === 0) return [sign, count > 1 ? ["0.", count - 1, upper ? "E0" : "e0"] : ["0e0"]];
    const kept = Math.min(count, $floatMaxDigits(size, f32));
    const [digits, point] = $floatExactDigits(size, kept);
    return [sign, $floatExpParts(digits, point, count, upper)];
  }
  if (precision !== undefined) {
    if (size === 0) return [sign, $floatZeroParts(precision)];
    const fixed = $floatFixedDigits(size, precision, $floatMaxDigits(size, f32));
    if (fixed === undefined) return [sign, $floatZeroParts(precision)];
    return [sign, $floatDecParts(fixed[0], fixed[1], precision)];
  }
  // `Debug`'s exponent where it's very large or small.
  const tiny = f32 ? Math.fround(1e-4) : 1e-4;
  if (trait === "G" && size !== 0 && (size >= 1e16 || size < tiny)) {
    const [digits, point] = $floatShortest(size, f32);
    return [sign, $floatExpParts(digits, point, 0, false)];
  }
  const least = trait === "G" ? 1 : 0;
  if (size === 0) return [sign, $floatZeroParts(least)];
  const [digits, point] = $floatShortest(size, f32);
  return [sign, $floatDecParts(digits, point, least)];
}
// Zero's parts, of `frac` digits after its point.
function $floatZeroParts(frac) {
  return frac > 0 ? ["0.", frac] : ["0"];
}
// `digits_to_dec_str`: `0.digits` times 10 to `point`, at least `frac`
// digits after its point.
function $floatDecParts(digits, point, frac) {
  if (point <= 0) {
    const parts = ["0.", -point, digits];
    if (frac > digits.length && frac - digits.length > -point) parts.push(frac - digits.length + point);
    return parts;
  }
  if (point < digits.length) {
    const parts = [digits.slice(0, point), ".", digits.slice(point)];
    if (frac > digits.length - point) parts.push(frac - (digits.length - point));
    return parts;
  }
  const parts = [digits, point - digits.length];
  if (frac > 0) parts.push(".", frac);
  return parts;
}
// `digits_to_exp_str`: `0.digits` times 10 to `point`, as `d.ddde-n`, at
// least `least` digits.
function $floatExpParts(digits, point, least, upper) {
  const parts = [digits.slice(0, 1)];
  if (digits.length > 1 || least > 1) {
    parts.push(".", digits.slice(1));
    if (least > digits.length) parts.push(least - digits.length);
  }
  const exp = point - 1;
  parts.push(exp < 0 ? (upper ? "E-" : "e-") : upper ? "E" : "e", String(Math.abs(exp)));
  return parts;
}
// The shortest digits of the positive `x` that read back as it, Rust's.
function $floatShortest(x, f32) {
  return f32 ? $f32Digits(x) : $f64Digits(x);
}
// `x`, positive, as an exact fraction, `[numerator, denominator]`, and
// the binary exponent `decode` gives it, which `estimate_max_buf_len` reads:
// `integer_decode`'s, one down, or two down where its significand is a
// power of two, a subnormal's as it is.
function $floatExact(x, f32) {
  const view = new DataView(new ArrayBuffer(8));
  let significand;
  let exponent2;
  let normal;
  let power;
  if (f32) {
    view.setFloat32(0, x);
    const bits = view.getUint32(0);
    const biased = bits >>> 23;
    const fraction = bits & 0x7fffff;
    significand = BigInt(biased === 0 ? fraction << 1 : fraction | 0x800000);
    exponent2 = biased - 150;
    normal = biased !== 0;
    power = fraction === 0;
  } else {
    view.setFloat64(0, x);
    const bits = view.getBigUint64(0);
    const biased = Number((bits >> 52n) & 2047n);
    const fraction = bits & ((1n << 52n) - 1n);
    significand = biased === 0 ? fraction << 1n : fraction | (1n << 52n);
    exponent2 = biased - 1075;
    normal = biased !== 0;
    power = fraction === 0n;
  }
  const numerator = exponent2 >= 0 ? significand << BigInt(exponent2) : significand;
  const denominator = exponent2 < 0 ? 1n << BigInt(-exponent2) : 1n;
  const decoded = !normal ? exponent2 : power ? exponent2 - 2 : exponent2 - 1;
  return [numerator, denominator, decoded];
}
// `estimate_max_buf_len`: how many digits `format_exact` makes at most.
function $floatMaxDigits(x, f32) {
  const decoded = $floatExact(x, f32)[2];
  return 21 + (((decoded < 0 ? -12 : 5) * decoded) >>> 4);
}
// `format_exact` of `count` digits: `[digits, point]`, rounded half to even.
function $floatExactDigits(x, count) {
  const [numerator, denominator] = $floatExact(x, false);
  const pow = (n) => 10n ** BigInt(n);
  let e = numerator.toString().length - denominator.toString().length;
  const below = (k) => (k >= 0 ? numerator < denominator * pow(k) : numerator * pow(-k) < denominator);
  if (below(e)) e--;
  const scale = count - 1 - e;
  const top = scale >= 0 ? numerator * pow(scale) : numerator;
  const bottom = scale >= 0 ? denominator : denominator * pow(-scale);
  let digits = top / bottom;
  const twice = 2n * (top % bottom);
  if (twice > bottom || (twice === bottom && digits % 2n === 1n)) digits += 1n;
  if (digits === pow(count)) {
    digits /= 10n;
    e++;
  }
  return [digits.toString(), e + 1];
}
// `to_exact_fixed_str`'s digits of `frac` after the point, rounded half to
// even, at most `most` of them: `[digits, point]`, or `undefined` where
// they round to zero.
function $floatFixedDigits(x, frac, most) {
  const [numerator, denominator] = $floatExact(x, false);
  const top = numerator * 10n ** BigInt(frac);
  let digits = top / denominator;
  const twice = 2n * (top % denominator);
  if (twice > denominator || (twice === denominator && digits % 2n === 1n)) digits += 1n;
  if (digits === 0n) return undefined;
  const text = digits.toString();
  const point = text.length - frac;
  return [text.slice(0, most), point];
}
