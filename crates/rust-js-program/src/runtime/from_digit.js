
function $fromDigit(num, radix) {
  if (radix > 36) throw $panic("from_digit: radix is too high (maximum 36)", $stdSites.fromDigit[0]);
  return num < radix ? String.fromCharCode(num < 10 ? 48 + num : 87 + num) : undefined;
}
