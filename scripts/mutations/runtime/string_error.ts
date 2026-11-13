// Mutations of src/runtime/string_error.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "string-error-debug-unquoted",
    breaks: "`{:?}` of a `Box<dyn Error>` made from a message shows it without quotes",
    file: "src/runtime/string_error.js",
    find: "    Debug: () => ({ fmt: (message) => $debugStr(message) }),",
    replace: "    Debug: () => ({ fmt: (message) => message }),",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "string-error-display-unpadded",
    breaks: "`{:>8}` of a `Box<dyn Error>` made from a message shows it unpadded, where std's `pad` pads it (#1689)",
    file: "src/runtime/string_error.js",
    find: "    Display: () => ({ fmt: (message, options) => $formatted(message, options), fmtInto: $writePadded }),",
    replace: "    Display: () => ({ fmt: (message) => message, fmtInto: $writePadded }),",
    tests: ["test/corpus.test.ts", "-t", "^std_error_messages_padded.rs$"],
  },
  {
    name: "string-error-written-whole",
    breaks: "a `Box<dyn Error>` of a message written into a writer under `{:>8}` writes its text whole, no fill char a `write_char` (#1652)",
    file: "src/runtime/string_error.js",
    find: "fmtInto: $writePadded }),",
    replace: "fmtInto: (f, message) => f.write(message) }),",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_generic_and_dyn.rs$"],
  },
];
