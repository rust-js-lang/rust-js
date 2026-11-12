// Mutations of src/runtime/json_read.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-read-swallows-errors",
    breaks: "`T::deserialize(d)` that fails is an `Ok` of nothing, and the impl goes on",
    file: "src/runtime/json_read.js",
    find: "    return { TAG: \"Err\", _0: e };\n",
    replace: "    return { TAG: \"Ok\", _0: undefined };\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Deserialize"],
  },
  {
    name: "json-try-catches-panics",
    breaks: "a panic inside a protocol call whose `Result` an impl looks at is taken for the backend's error, an `Err`, where Rust's panic unwinds on (#1545)",
    file: "src/runtime/json_read.js",
    find: "    if (!(e instanceof $JsonError)) throw e;\n",
    replace: "",
    tests: ["test/serde-protocol.test.ts"],
  },
];
