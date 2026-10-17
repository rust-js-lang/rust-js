// `a * b + c` rounded once, as a fused multiply-add is (ADR 0430), to a
// float of `digits` significant bits whose least step is `2 ** least`: an
// `f64`'s 53 and -1074, an `f32`'s 24 and -149. Each operand is an integer
// times a power of two, so the sum is one too, exactly, in a `BigInt`.
function $fused(a, b, c, digits, least) {
  // Of an infinity or a NaN, and of a sum that's exactly zero, whose sign
  // JS's own gives, JS's answer is the one rounding gives.
  if (!Number.isFinite(a) || !Number.isFinite(b) || !Number.isFinite(c)) {
    return a * b + c;
  }
  const parts = (x) => {
    const view = new DataView(new ArrayBuffer(8));
    view.setFloat64(0, x);
    const high = view.getUint32(0);
    const field = (high >>> 20) & 0x7ff;
    let whole = (BigInt(high & 0xfffff) << 32n) | BigInt(view.getUint32(4));
    if (field !== 0) {
      whole |= 1n << 52n;
    }
    return [high >>> 31 ? -whole : whole, Math.max(field, 1) - 1075];
  };
  const [ma, ea] = parts(a);
  const [mb, eb] = parts(b);
  const [mc, ec] = parts(c);
  const ep = ea + eb;
  const exponent = Math.min(ep, ec);
  let sum = ((ma * mb) << BigInt(ep - exponent)) + (mc << BigInt(ec - exponent));
  if (sum === 0n) {
    return a * b + c;
  }
  const negative = sum < 0n;
  if (negative) {
    sum = -sum;
  }
  // The step its digits are counted in: its top `digits` bits, or the
  // least there is.
  const top = exponent + sum.toString(2).length;
  const step = Math.max(top - digits, least);
  if (step > exponent) {
    const shift = BigInt(step - exponent);
    const half = 1n << (shift - 1n);
    const rest = sum & ((1n << shift) - 1n);
    sum >>= shift;
    // To nearest, a tie to even.
    if (rest > half || (rest === half && (sum & 1n) === 1n)) {
      sum += 1n;
    }
  }
  // Exact, as `2 ** at` is from the least step up, unless it's past the
  // float's range, where it's infinite, as rounding makes it.
  const value = Number(sum) * 2 ** Math.max(step, exponent);
  return negative ? -value : value;
}
