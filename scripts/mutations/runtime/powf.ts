// Mutations of src/runtime/powf.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "powf-one-to-nan",
    breaks: "`1.0.powf(f64::NAN)` and `(-1.0).powf(f64::INFINITY)` are NaN, where Rust's are 1",
    file: "src/runtime/powf.js",
    find: "  return a === 1 || (a === -1 && Math.abs(b) === Infinity) ? 1 : a ** b;",
    replace: "  return a ** b;",
    tests: ["test/corpus.test.ts", "-t", "matrix_floats"],
  },
];
