// Mutations of src/runtime/big_to_f32.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "big-to-f32-past-max-finite",
    breaks: "a `u128` that rounds up to 2^128 as an `f32` is that double, past `f32::MAX`, not infinity",
    file: "src/runtime/big_to_f32.js",
    find: "  const x = Math.fround(Number(size));\n",
    replace: "  const x = Number(size);\n",
    tests: ["test/corpus.test.ts", "-t", "^big_to_f32_overflow.rs$"],
  },
];
