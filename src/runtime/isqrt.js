// `x.isqrt()` of an integer, a number or a BigInt: its square root, rounded
// down. A BigInt's starts from a float's guess, which it corrects.
function $isqrt(x) {
  if (x < 0) throw new Error("argument of integer square root cannot be negative");
  if (typeof x !== "bigint") return Math.floor(Math.sqrt(x));
  let root = BigInt(Math.floor(Math.sqrt(Number(x))));
  while (root * root > x) root--;
  while ((root + 1n) * (root + 1n) <= x) root++;
  return root;
}
