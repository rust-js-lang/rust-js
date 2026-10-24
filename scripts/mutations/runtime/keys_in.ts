// Mutations of src/runtime/keys_in.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "keys-in-own-keys",
    breaks: "`js::keys_in(o)` as an iterator is `Object.keys(o)`'s, without what `o` inherits",
    file: "src/runtime/keys_in.js",
    find: "  for (const key in object) {",
    replace: "  for (const key of Object.keys(object)) {",
    tests: ["test/js-object.test.ts", "-t", "a for loop of keys_in"],
  },
];
