// Mutations of src/runtime/json_value.js (ADR 0083).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-get-mut-unwritten",
    breaks: "a handle `get_mut` gives of a `Value`'s entry writes nothing back",
    file: "src/runtime/json_value.js",
    find: "set value(next) { items.set(key, next); }",
    replace: "set value(next) {}",
    tests: ["test/serde.test.ts", "-t", "changes to and from Null"],
  },
  {
    name: "value-writer-unrewound",
    breaks: "a `Value`'s writer keeps what it opened of a value whose write failed and was caught, so the value after is built inside it, where serde_json builds each apart and drops a failed one (#1545)",
    file: "src/runtime/json_value.js",
    find: "      stack.length = depth;\n",
    replace: "",
    tests: ["test/serde-protocol.test.ts"],
  },
];
