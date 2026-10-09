// Mutations of src/runtime/to_fixed.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "to-fixed-tie-away",
    breaks: "`format!(\"{x:.0}\")` of 2.5 is \"3\", a tie rounded away from zero as JS's `toFixed` rounds it, not to even as Rust does",
    file: "src/runtime/to_fixed.js",
    find: "  if (twice > denominator || (twice === denominator && rounded % 2n === 1n)) rounded += 1n;",
    replace: "  if (twice >= denominator) rounded += 1n;",
    tests: ["test/compiler.test.ts", "-t", "the builtins crate's number::to_fixed is JS's toFixed, not format!'s"],
  },
];
