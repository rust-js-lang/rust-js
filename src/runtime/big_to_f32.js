
function $bigToF32(n) {
  const negative = n < 0n;
  let size = negative ? -n : n;
  const length = size.toString(2).length;
  if (length > 24) {
    const shift = BigInt(length - 24);
    const rest = size & ((1n << shift) - 1n);
    const half = 1n << (shift - 1n);
    size >>= shift;
    if (rest > half || (rest === half && (size & 1n) === 1n)) size++;
    size <<= shift;
  }
  // Rounded up to 2^128, past `f32::MAX`: infinity, as `fround` makes it;
  // any other is an `f32` already.
  const x = Math.fround(Number(size));
  return negative ? -x : x;
}
