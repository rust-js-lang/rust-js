// Mutations of src/runtime/rotate_bits.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rotate-bits-right-leftwards",
    breaks: "`rotate_right` rotates left",
    file: "src/runtime/rotate_bits.js",
    find: "  const up = left ? by : bits - by;",
    replace: "  const up = by;",
    tests: ["test/corpus.test.ts", "-t", "integer_bits"],
  },
];
