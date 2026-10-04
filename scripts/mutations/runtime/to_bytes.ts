// Mutations of src/runtime/to_bytes.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "to-bytes-order-flipped",
    breaks: "`to_be_bytes` of a `u32` is little-endian, and `to_le_bytes` big",
    file: "src/runtime/to_bytes.js",
    find: "  else if (size === 4) view.setUint32(0, x, little);",
    replace: "  else if (size === 4) view.setUint32(0, x, !little);",
    tests: ["test/corpus.test.ts", "-t", "integer_bits"],
  },
  {
    name: "to-bytes-128-halves-swapped",
    breaks: "a 128-bit integer's bytes put its halves the wrong way round",
    file: "src/runtime/to_bytes.js",
    find: "    view.setBigUint64(little ? 0 : 8, BigInt.asUintN(64, x), little);",
    replace: "    view.setBigUint64(little ? 8 : 0, BigInt.asUintN(64, x), little);",
    tests: ["test/corpus.test.ts", "-t", "integers_128"],
  },
];
