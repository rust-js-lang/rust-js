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
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_generic_and_dyn.rs$"],
  },
  {
    name: "parse-error-dyn-debug-written-whole",
    breaks: "a parse error in a `Box<dyn Error>` written by its `Debug` into a writer is one `write_str`, where std's builders write its name, braces, field and kind apart (#2250)",
    file: "src/runtime/parse_error_dyn.js",
    find: "      fmtInto: (f, value, options) => $writeDebugParseError(f, value, name, options?.alternate),\n",
    replace: "      fmtInto: (f, value, options) => f.write($debugParseError(value, name, options)),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_error_debug_written_piecewise.rs$"],
  },
  {
    name: "parse-error-dyn-debug-unpretty",
    breaks: "`{:#?}` of a parse error in a `Box<dyn Error>` is on one line, as its dictionary's `fmt` drops the `Formatter`'s options (#2250)",
    file: "src/runtime/parse_error_dyn.js",
    find: "      fmt: (value, options) => $debugParseError(value, name, options),\n",
    replace: "      fmt: (value) => $debugParseError(value, name),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_error_debug_written_piecewise.rs$"],
  },
];
