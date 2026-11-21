// Mutations of src/runtime/json_error.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-error-written-whole",
    breaks: "serde_json's error with a line written into a writer writes only its message, where its `write!` writes five pieces (#1652)",
    file: "src/runtime/json_error.js",
    find: "  if (e.line === 0) return;\n",
    replace: "  return;\n",
    tests: ["test/writer-sink.test.ts", "-t", "serde_json's error and an OS error"],
  },
  {
    name: "json-error-debug-written-whole",
    breaks: "serde_json's error in a `Box<dyn Error>` written by its `Debug` into a writer writes its message's `Debug` as one `write_str`, where a `String`'s `Debug` writes each escape apart (#2250)",
    file: "src/runtime/json_error.js",
    find: "  $writeDebugStr(f, e.message);\n",
    replace: "  f.write($debugStr(e.message));\n",
    tests: ["test/serde-values.test.ts", "-t", "written into a writer a piece at a time"],
  },
];
