// Mutations of src/runtime/parse_error_dyn.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dyn-parse-error-unpadded",
    breaks: "a parse error held as a `dyn Error` is shown by `{}` as its text alone, a placeholder's width dropped",
    file: "src/runtime/parse_error_dyn.js",
    find: "    Display: () => ({ fmt: (value, options) => $formatted(value, options), fmtInto: $writePadded }),\n",
    replace: "    Display: () => ({ fmt: (value) => value, fmtInto: $writePadded }),\n",
    tests: ["test/corpus.test.ts", "-t", "^display_options_handed_on.rs$"],
  },
  {
    name: "parse-error-dyn-written-whole",
    breaks: "a parse error made a `Box<dyn Error>`, written into a writer under `{:>8}`, writes its text whole (#1652)",
    file: "src/runtime/parse_error_dyn.js",
    find: "fmtInto: $writePadded }),",
    replace: "fmtInto: (f, value) => f.write(value) }),",
    tests: ["test/writer-sink.test.ts", "-t", "each of generic code's and dyns' writes is native Rust's, in the plain build"],
  },
];
