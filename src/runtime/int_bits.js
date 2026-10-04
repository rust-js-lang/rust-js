// An integer's bits, as num-traits' `PrimInt` asks of every width: each of
// `bits` bits, as an unsigned BigInt's binary digits, and back to the kind
// it was, a number or a BigInt, `signed` or not.
function $intDigits(x, bits) {
  return BigInt.asUintN(bits, BigInt(x)).toString(2).padStart(bits, "0");
}

function $intOfDigits(digits, bits, signed, big) {
  const unsigned = BigInt(`0b${digits}`);
  const n = signed ? BigInt.asIntN(bits, unsigned) : unsigned;
  return big ? n : Number(n);
}

// `x.leading_ones()`, or `trailing_ones()` (`trailing`).
function $edgeOnes(x, bits, trailing = false) {
  const digits = $intDigits(x, bits);
  return (trailing ? digits.match(/1*$/) : digits.match(/^1*/))[0].length;
}

// `x.swap_bytes()`: its bytes the other way round.
function $swapBytes(x, bits, signed) {
  const digits = $intDigits(x, bits);
  const bytes = digits.match(/.{8}/g).reverse().join("");
  return $intOfDigits(bytes, bits, signed, typeof x === "bigint");
}

// `x.reverse_bits()`: its bits the other way round.
function $reverseBits(x, bits, signed) {
  const digits = [...$intDigits(x, bits)].reverse().join("");
  return $intOfDigits(digits, bits, signed, typeof x === "bigint");
}

// `a.checked_div_euclid(b)`, or `checked_rem_euclid` (`rem`): `undefined`,
// `None`, where `b` is 0 or the quotient overflows, `MIN / -1`.
function $checkedEuclid(a, b, min, rem) {
  const [zero, one] = typeof a === "bigint" ? [0n, 1n] : [0, 1];
  if (b === zero || (min !== undefined && a === min && b === -one)) return undefined;
  let q = typeof a === "bigint" ? a / b : Math.trunc(a / b);
  let r = a - q * b;
  if (r < zero) {
    q = b > zero ? q - one : q + one;
    r = b > zero ? r + b : r - b;
  }
  return rem ? r : q;
}
