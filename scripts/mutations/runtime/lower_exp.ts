// Mutations of src/runtime/lower_exp.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lower-exp-bigint-infinite",
    breaks: "`{:e}` of a 64-bit integer, a BigInt, is `inf`",
    file: "src/runtime/lower_exp.js",
    find: '  if (precision !== undefined) return $lowerExpRounded(x, precision);\n  if (typeof x === "bigint") {',
    replace: '  if (precision !== undefined) return $lowerExpRounded(x, precision);\n  if (typeof x === "bigint" && false) {',
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
  {
    name: "exp-precision-ties-up",
    breaks: "`{:.1e}` of `1.25` rounds its tie up, `1.3e0`, where Rust rounds half to even",
    file: "src/runtime/lower_exp.js",
    find: "  if (twice > bottom || (twice === bottom && digits % 2n === 1n)) digits += 1n;\n",
    replace: "  if (twice >= bottom) digits += 1n;\n",
    tests: ["test/corpus.test.ts", "-t", "^exp_precision.rs$"],
  },
  {
    name: "exp-precision-carry-kept",
    breaks: "`{:.1e}` of `9.96` is `10.0e0`, not `1.0e1`",
    file: "src/runtime/lower_exp.js",
    find: "  if (digits === pow(precision + 1)) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^exp_precision.rs$"],
  },
  {
    name: "exp-precision-exponent-off",
    breaks: "`{:.2e}` of a number whose digits' lengths say one more than its exponent, `0.125`'s, is one off",
    file: "src/runtime/lower_exp.js",
    find: "  if (below(e)) e--;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^exp_precision.rs$"],
  },
];
