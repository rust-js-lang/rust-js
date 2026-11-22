// A 64-bit integer's bits, counted; a 128-bit one's, of `bits` 128.
function $bigCountOnes(x, bits = 64) {
  return BigInt.asUintN(bits, x).toString(2).replaceAll("0", "").length;
}
function $bigLeadingZeros(x, bits = 64) {
  const unsigned = BigInt.asUintN(bits, x);
  return unsigned === 0n ? bits : bits - unsigned.toString(2).length;
}
function $bigTrailingZeros(x, bits = 64) {
  const digits = BigInt.asUintN(bits, x).toString(2);
  return x === 0n ? bits : digits.length - 1 - digits.lastIndexOf("1");
}
