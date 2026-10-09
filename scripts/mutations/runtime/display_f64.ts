// Mutations of src/runtime/display_f64.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "display-f64-negative-zero-unsigned",
    breaks: "`-0.0` is shown as `0`, where Rust shows `-0`",
    file: "src/runtime/display_f64.js",
    find: "  const sign = value < 0 || Object.is(value, -0) ? '-' : '';",
    replace: "  const sign = value < 0 ? '-' : '';",
    tests: ["test/traits.test.ts", "-t", "Rust f64 Display agrees with native output"],
  },
];
