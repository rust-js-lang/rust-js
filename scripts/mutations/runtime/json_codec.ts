// Mutations of src/runtime/json_codec.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-codec-swallows-errors",
    breaks: "a hand-written impl's `Err` is read as its value, and serde_json's error never comes",
    file: "src/runtime/json_codec.js",
    find: "    if (result.TAG === \"Ok\") return result._0;\n",
    replace: "    return result._0;\n",
    tests: ["test/serde.test.ts", "-t", "hand-written"],
  },
];
