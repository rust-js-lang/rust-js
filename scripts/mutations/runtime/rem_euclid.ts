// Mutations of src/runtime/rem_euclid.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "wrapping-rem-euclid-panics",
    breaks: "`MIN.wrapping_rem_euclid(-1)` panics as `rem_euclid` does, not 0",
    file: "src/runtime/rem_euclid.js",
    find: "  if (a === min && b == -1) return typeof a === \"bigint\" ? 0n : 0;\n  return typeof a === \"bigint\" ? $bigRemEuclid",
    replace: "  return typeof a === \"bigint\" ? $bigRemEuclid",
    tests: ["test/corpus.test.ts", "-t", "^int_shift_multiple_methods.rs$"],
  },
  {
    name: "overflowing-euclid-never-wraps",
    breaks: "`i8::MIN.overflowing_div_euclid(-1)` says it didn't wrap, `(-128, false)`, where std says `(-128, true)` (#940)",
    file: "src/runtime/rem_euclid.js",
    find: "  return [made, a === min && b == -1];\n",
    replace: "  return [made, false];\n",
    tests: ["test/corpus.test.ts", "-t", "^overflowing_euclid.rs$"],
  },
];
