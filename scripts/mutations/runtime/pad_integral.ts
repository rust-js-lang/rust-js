// Mutations of src/runtime/pad_integral.js (ADR 0143).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pad-integral-zeros-before-prefix",
    breaks: "`{:#010}` of `pad_integral`'s digits puts its zeros before the prefix",
    file: "src/runtime/pad_integral.js",
    find: "  if (options.zero) return head + digits.padStart(width - [...head].length, \"0\");\n",
    replace: "  if (options.zero) return (head + digits).padStart(width, \"0\");\n",
    tests: ["test/corpus.test.ts", "-t", "^pad_integral.rs$"],
  },
  {
    name: "pad-integral-plus-ignored",
    breaks: "`{:+}` of `pad_integral` writes no `+`",
    file: "src/runtime/pad_integral.js",
    find: "  const sign = !nonNegative ? \"-\" : options.plus ? \"+\" : \"\";\n",
    replace: "  const sign = !nonNegative ? \"-\" : \"\";\n",
    tests: ["test/corpus.test.ts", "-t", "^pad_integral.rs$"],
  },
];
