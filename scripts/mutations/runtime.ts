// Mutations of src/runtime.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "pure-helpers-act",
    breaks: "the runtime's helpers that only read, `$jsonIndex(v, \"a\")`, count as doing something, held apart from MIR",
    file: "src/runtime.rs",
    find: "    \"$jsonIndex\",\n",
    replace: "",
    tests: ["test/serde.test.ts","-t","a Value's reads"],
  },
];
