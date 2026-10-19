// Mutations of src/runtime/float_euclid.js (ADR 0064).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "float-rem-euclid-negative",
    breaks: "`(-7.5f64).rem_euclid(2.0)` is the remainder, -1.5, not made non-negative",
    file: "src/runtime/float_euclid.js",
    find: "  if (rem) return r < 0 ? r + Math.abs(b) : r;\n",
    replace: "  if (rem) return r;\n",
    tests: ["test/corpus.test.ts", "-t", "^float_methods_more.rs$"],
  },
  {
    name: "float-div-euclid-truncated",
    breaks: "`(-7.5f64).div_euclid(2.0)` is the truncated quotient, -3, not -4",
    file: "src/runtime/float_euclid.js",
    find: "  if (r < 0) return b > 0 ? q - 1 : q + 1;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^float_methods_more.rs$"],
  },
  {
    name: "abs-sub-negative",
    breaks: "`2f64.abs_sub(5.0)` is -3, not 0",
    file: "src/runtime/float_euclid.js",
    find: "  return a > b ? a - b : 0;\n",
    replace: "  return a - b;\n",
    tests: ["test/corpus.test.ts", "-t", "^float_methods_more.rs$"],
  },
];
