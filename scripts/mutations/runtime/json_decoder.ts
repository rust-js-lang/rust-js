// Mutations of src/runtime/json_decoder.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-f32-integer-through-f64",
    breaks: "an integer in JSON read as an `f32` is rounded to an `f64` first, and then can be a tie it isn't",
    file: "src/runtime/json_decoder.js",
    find: ": $bigToF32(BigInt(n.value))));",
    replace: ": Math.fround(Number(n.value))));",
    tests: ["test/serde.test.ts", "-t", "an f32 is written"],
  },
];
