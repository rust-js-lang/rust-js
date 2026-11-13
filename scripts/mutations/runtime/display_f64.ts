// Mutations of src/runtime/display_f64.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "display-f64-negative-zero-unsigned",
    breaks: "`-0.0` is shown as `0`, where Rust shows `-0`",
    file: "src/runtime/display_f64.js",
    find: '  const sign = value < 0 || Object.is(value, -0) ? "-" : "";',
    replace: '  const sign = value < 0 ? "-" : "";',
    tests: ["test/traits.test.ts", "-t", "Rust f64 Display agrees with native output"],
  },
  {
    name: "f64-digits-tie-to-the-smaller",
    breaks: "of two shortest digit strings equally near an `f64`, the smaller is taken, `1888570120608320.2`, where Rust takes the larger (#1682)",
    file: "src/runtime/display_f64.js",
    find: "(distance === bestDistance && candidate > best)",
    replace: "(distance === bestDistance && candidate < best)",
    tests: ["test/corpus.test.ts", "-t", "^float_shortest_digits_tie_to_the_larger.rs$"],
  },
];
