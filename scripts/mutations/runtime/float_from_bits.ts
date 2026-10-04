// Mutations of src/runtime/float_from_bits.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "f32-from-bits-integer",
    breaks: "`f32::from_bits` gives its bits back as an integer",
    file: "src/runtime/float_from_bits.js",
    find: "  return view.getFloat32(0);",
    replace: "  return view.getUint32(0);",
    tests: ["test/corpus.test.ts", "-t", "integer_bits"],
  },
];
