// Mutations of src/runtime/json_read.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-read-swallows-errors",
    breaks: "`T::deserialize(d)` that fails is an `Ok` of nothing, and the impl goes on",
    file: "src/runtime/json_read.js",
    find: "    if (e instanceof $JsonError) return { TAG: \"Err\", _0: e };\n",
    replace: "    if (e instanceof $JsonError) return { TAG: \"Ok\", _0: undefined };\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Deserialize"],
  },
];
