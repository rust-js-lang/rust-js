// `x.pow(e)` of a 64-bit integer, squared as Rust squares it, wrapped each
// time; of a 128-bit one, of `bits` 128.
function $bigPow(base, exp, bits = 64) {
  let result = 1n;
  base = BigInt.asUintN(bits, base);
  for (let e = exp; e > 0; e >>>= 1) {
    if (e & 1) result = BigInt.asUintN(bits, result * base);
    base = BigInt.asUintN(bits, base * base);
  }
  return result;
}
