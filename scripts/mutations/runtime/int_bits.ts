// Mutations of src/runtime/int_bits.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "edge-ones-from-one-end",
    breaks: "`trailing_ones()` counts the leading ones",
    file: "src/runtime/int_bits.js",
    find: "  return (trailing ? digits.match(/1*$/) : digits.match(/^1*/))[0].length;",
    replace: "  return digits.match(/^1*/)[0].length;",
    tests: ["test/corpus.test.ts", "-t", "bit_methods"],
  },
  {
    name: "swap-bytes-kept",
    breaks: "`swap_bytes()` keeps its bytes in order",
    file: "src/runtime/int_bits.js",
    find: '  const bytes = digits.match(/.{8}/g).reverse().join("");',
    replace: '  const bytes = digits.match(/.{8}/g).join("");',
    tests: ["test/corpus.test.ts", "-t", "bit_methods"],
  },
  {
    name: "swap-bytes-unsigned",
    breaks: "`(-2i32).swap_bytes()` is read back unsigned",
    file: "src/runtime/int_bits.js",
    find: "  const n = signed ? BigInt.asIntN(bits, unsigned) : unsigned;",
    replace: "  const n = unsigned;",
    tests: ["test/corpus.test.ts", "-t", "bit_methods"],
  },
  {
    name: "checked-euclid-overflow",
    breaks: "`i8::MIN.checked_div_euclid(-1)` is `Some`, not `None`",
    file: "src/runtime/int_bits.js",
    find: "  if (b === zero || (min !== undefined && a === min && b === -one)) return undefined;",
    replace: "  if (b === zero) return undefined;",
    tests: ["test/corpus.test.ts", "-t", "bit_methods"],
  },
  {
    name: "checked-euclid-truncating",
    breaks: "`(-7).checked_rem_euclid(2)` is the truncating `-1`",
    file: "src/runtime/int_bits.js",
    find: "  if (r < zero) {",
    replace: "  if (false) {",
    tests: ["test/corpus.test.ts", "-t", "bit_methods"],
  },
];
