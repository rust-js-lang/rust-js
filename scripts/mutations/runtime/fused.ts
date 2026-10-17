// Mutations of src/runtime/fused.js (ADR 0430).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fused-tie-up",
    breaks: "`mul_add`'s tie rounds up, not to even, and half the least subnormal is it, not 0",
    file: "src/runtime/fused.js",
    find: "    if (rest > half || (rest === half && (sum & 1n) === 1n)) {\n",
    replace: "    if (rest >= half) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mul_add.rs$"],
  },
  {
    name: "fused-past-tie-down",
    breaks: "`mul_add` past a tie rounds down, as rounding `a * b` first would",
    file: "src/runtime/fused.js",
    find: "    if (rest > half || (rest === half && (sum & 1n) === 1n)) {\n",
    replace: "    if (rest === half && (sum & 1n) === 1n) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mul_add.rs$"],
  },
  {
    name: "fused-sign-lost",
    breaks: "a negative `mul_add` is positive",
    file: "src/runtime/fused.js",
    find: "  return negative ? -value : value;\n",
    replace: "  return value;\n",
    tests: ["test/corpus.test.ts", "-t", "^mul_add.rs$"],
  },
  {
    name: "fused-zero-unsigned",
    breaks: "`(-0.0).mul_add(1.0, -0.0)` is `0`, not `-0`",
    file: "src/runtime/fused.js",
    find: "  if (sum === 0n) {\n    return a * b + c;\n  }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mul_add.rs$"],
  },
  {
    name: "fused-infinity-as-number",
    breaks: "an infinity or a NaN given `mul_add` is taken for a finite number",
    file: "src/runtime/fused.js",
    find: "  if (!Number.isFinite(a) || !Number.isFinite(b) || !Number.isFinite(c)) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mul_add.rs$"],
  },
  {
    name: "fused-subnormal-hidden-bit",
    breaks: "a subnormal given `mul_add` has a leading bit it hasn't",
    file: "src/runtime/fused.js",
    find: "    if (field !== 0) {\n",
    replace: "    if (true) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mul_add.rs$"],
  },
];
