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
];
