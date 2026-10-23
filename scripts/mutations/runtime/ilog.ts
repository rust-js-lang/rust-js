// Mutations of src/runtime/ilog.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ilog-off-at-power",
    breaks: "`1000u32.ilog10()` is 2: an exact power isn't counted",
    file: "src/runtime/ilog.js",
    find: "    for (; x >= base; x = Math.floor(x / base)) log++;",
    replace: "    for (; x > base; x = Math.floor(x / base)) log++;",
    tests: ["test/corpus.test.ts", "-t", "number_methods"],
  },
  {
    name: "checked-ilog-panics",
    breaks: "`checked_ilog10()` of zero panics, not `None`",
    file: "src/runtime/ilog.js",
    find: "    if (checked) return undefined;\n    throw new Error(\"argument of integer logarithm must be positive\");\n",
    replace: "    throw new Error(\"argument of integer logarithm must be positive\");\n",
    tests: ["test/corpus.test.ts", "-t", "^number_methods.rs$"],
  },
  {
    name: "ilog-base-unchecked",
    breaks: "`ilog(1)` loops or counts, not panicking, and `checked_ilog(1)` isn't none",
    file: "src/runtime/ilog.js",
    find: "  if (base < 2) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^int_overflow_sign_methods.rs$"],
  },
];
