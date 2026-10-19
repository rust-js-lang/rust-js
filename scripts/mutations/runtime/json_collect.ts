// Mutations of src/runtime/json_collect.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-collect-seq-unseparated",
    breaks: "`collect_seq`'s items are written with no separator between them",
    file: "src/runtime/json_collect.js",
    find: "    json.element();\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "collects and writes variants"],
  },
];
