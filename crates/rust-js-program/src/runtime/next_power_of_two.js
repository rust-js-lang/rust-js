// `n.next_power_of_two()` of a `bits`-bit unsigned number, a JS number or a
// `bigint`: the least power of two at least `n`, or `undefined` past `bits`,
// `checked_next_power_of_two()`'s `None`, where the other wraps to `0`.
function $nextPowerOfTwo(n, bits) {
  if (typeof n === "bigint") {
    let p = 1n;
    while (p < n) p <<= 1n;
    return p >> BigInt(bits) === 0n ? p : undefined;
  }
  let p = 1;
  while (p < n) p *= 2;
  return p < 2 ** bits ? p : undefined;
}
