// Mutations of src/runtime/wrapping_div.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "wrapping-div-min",
    breaks: "`i32::MIN.wrapping_div(-1)` is 2^31, out of range, not `MIN`",
    file: "src/runtime/wrapping_div.js",
    find: "  if (a === min && b == -1) return a;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "integer_families"],
  },
  {
    name: "saturating-div-wraps",
    breaks: "`MIN.saturating_div(-1)` wraps to `MIN`, not `MAX`",
    file: "src/runtime/wrapping_div.js",
    find: "  return a === min && b == -1 ? max : $wrappingDiv(a, b, min);\n",
    replace: "  return $wrappingDiv(a, b, min);\n",
    tests: ["test/corpus.test.ts", "-t", "^int_div_sign_methods.rs$"],
  },
  {
    name: "is-multiple-of-zero-true",
    breaks: "`x.is_multiple_of(0)` is true of any `x`",
    file: "src/runtime/wrapping_div.js",
    find: "  return b == 0 ? a == 0 : a % b == 0;\n",
    replace: "  return b == 0 ? true : a % b == 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^int_div_sign_methods.rs$"],
  },
  {
    name: "next-multiple-unwrapped",
    breaks: "`next_multiple_of` past its range isn't wrapped",
    file: "src/runtime/wrapping_div.js",
    find: "  return next > max ? next - max - (typeof next === \"bigint\" ? 1n : 1) : next;\n",
    replace: "  return next;\n",
    tests: ["test/corpus.test.ts", "-t", "^int_shift_multiple_methods.rs$"],
  },
];
