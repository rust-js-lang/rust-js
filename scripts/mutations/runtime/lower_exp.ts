// Mutations of src/runtime/lower_exp.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lower-exp-bigint-infinite",
    breaks: "`{:e}` of a 64-bit integer, a BigInt, is `inf`",
    file: "src/runtime/lower_exp.js",
    find: '  if (typeof x === "bigint") {',
    replace: '  if (typeof x === "bigint" && false) {',
    tests: ["test/corpus.test.ts", "-t", "exp_format"],
  },
  {
    name: "lower-exp-zero-unsigned",
    breaks: "`{:e}` of `-0.0` is `0e0`, its sign dropped",
    file: "src/runtime/lower_exp.js",
    find: '  const sign = x < 0 || Object.is(x, -0) ? "-" : "";',
    replace: '  const sign = x < 0 ? "-" : "";',
    tests: ["test/corpus.test.ts", "-t", "exp_format"],
  },
];
