// Mutations of src/runtime/json_de_error.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-unexpected-float-pointless",
    breaks: "`Unexpected::Float(1000.0)` shows `1000`, where serde shows `1000.0`",
    file: "src/runtime/json_de_error.js",
    find: "Number.isFinite(u._0) && !shown.includes(\".\") ? `${shown}.0` : shown",
    replace: "shown",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "json-unknown-two-as-many",
    breaks: "of three names, serde's `one of` is written as of two",
    file: "src/runtime/json_de_error.js",
    find: "names.length === 2 ? `${quoted[0]} or ${quoted[1]}` : `one of ${quoted.join(\", \")}`",
    replace: "`${quoted[0]} or ${quoted[1]}`",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
];
