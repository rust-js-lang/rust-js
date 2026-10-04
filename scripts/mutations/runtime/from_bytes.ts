// Mutations of src/runtime/from_bytes.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "from-bytes-unsigned",
    breaks: "`i16::from_le_bytes` reads its bytes as a `u16`, never negative",
    file: "src/runtime/from_bytes.js",
    find: "  if (size === 2) return signed ? view.getInt16(0, little) : view.getUint16(0, little);",
    replace: "  if (size === 2) return view.getUint16(0, little);",
    tests: ["test/corpus.test.ts", "-t", "integer_bits"],
  },
  {
    name: "from-bytes-128-unsigned",
    breaks: "an `i128` from bytes is read unsigned, `-3` as `2^128 - 3`",
    file: "src/runtime/from_bytes.js",
    find: "    return signed ? BigInt.asIntN(128, n) : n;",
    replace: "    return n;",
    tests: ["test/corpus.test.ts", "-t", "integers_128"],
  },
];
