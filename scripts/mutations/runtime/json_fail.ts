// Mutations of src/runtime/json_fail.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-float-js-notation",
    breaks: "a float is written in fixed notation where JS uses it, up to 1e21, `12345678901234567000.0`, where serde_json writes `1.2345678901234567e+19`",
    file: "src/runtime/json_fail.js",
    find: "  if (e < -5 || e > 15) return",
    replace: "  if (e < -7 || e > 20) return",
    tests: ["test/serde.test.ts", "-t", "numbers are written and read as serde_json does"],
  },
];
