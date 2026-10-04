// Mutations of src/runtime/debug.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "debug-string-as-json",
    breaks: "a string error's `{:?}` in an `unwrap()` message is JSON's, `\\u0007`, not Rust's `\\u{7}`",
    file: "src/runtime/debug.js",
    find: "    return $debugStr(v);",
    replace: "    return JSON.stringify(v);",
    tests: ["test/corpus.test.ts", "-t", "unwrap_string_debug"],
  },
];
