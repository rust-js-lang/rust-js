// Mutations of src/runtime/parse_error_dyn.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dyn-parse-error-unpadded",
    breaks: "a parse error held as a `dyn Error` is shown by `{}` as its text alone, a placeholder's width dropped",
    file: "src/runtime/parse_error_dyn.js",
    find: "    Display: () => ({ fmt: (value, options) => $formatted(value, options) }),\n",
    replace: "    Display: () => ({ fmt: (value) => value }),\n",
    tests: ["test/corpus.test.ts", "-t", "^display_options_handed_on.rs$"],
  },
];
