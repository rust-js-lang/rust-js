// `x.rotate_left(n)`, or `rotate_right(n)`, of an integer's unsigned bits,
// `bits` of them: what's shifted out of one end comes in at the other. A
// number's by powers of two, which are exact; a BigInt's by shifts.
function $rotateBits(x, n, bits, left) {
  const by = n % bits;
  if (by === 0) return x;
  const up = left ? by : bits - by;
  if (typeof x === "bigint") {
    const size = BigInt(bits);
    const shift = BigInt(up);
    return ((x << shift) | (x >> (size - shift))) & ((1n << size) - 1n);
  }
  return ((x * 2 ** up) % 2 ** bits) + Math.floor(x / 2 ** (bits - up));
}
