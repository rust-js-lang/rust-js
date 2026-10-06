// Mutations of src/runtime/dict_get.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dict-get-inherited",
    breaks: "`dict::get(fields, \"toString\")` is `Object.prototype`'s, where a JSON object has no such key",
    file: "src/runtime/dict_get.js",
    find: "Object.hasOwn(dict, key)",
    replace: "key in dict",
    tests: ["test/compiler.test.ts", "-t", "JSON is a typed value"],
  },
  {
    name: "dict-get-unboxed",
    breaks: "a JSON `null`'s `dict::get` is `None`, as a key that isn't there is, not `Some(None)`",
    file: "src/runtime/dict_get.js",
    find: "$some(dict[key])",
    replace: "dict[key]",
    tests: ["test/compiler.test.ts", "-t", "JSON is a typed value"],
  },
];
