// Mutations of src/runtime/float_midpoint.js (ADR 0064).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "float-midpoint-overflows",
    breaks: "`f64::MAX.midpoint(f64::MAX)` is the sum halved, `inf`",
    file: "src/runtime/float_midpoint.js",
    find: "  if (Math.abs(a) <= hi && Math.abs(b) <= hi) return (a + b) / 2;\n",
    replace: "  return (a + b) / 2;\n",
    tests: ["test/corpus.test.ts", "-t", "^float_methods_more.rs$"],
  },
];
