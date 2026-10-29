// Mutations of src/runtime/id.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "id-of-each-object-one",
    breaks: "every object is known by one number in a key, so two fibers alike are one key (#398)",
    file: "src/runtime/id.js",
    find: "  if (id === undefined) $ids.set(object, (id = ++$lastId));\n",
    replace: "  if (id === undefined) $ids.set(object, (id = 1));\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "id-of-one-object-new",
    breaks: "one object is known by another number each time, so one fiber is two keys (#398)",
    file: "src/runtime/id.js",
    find: "  let id = $ids.get(object);\n",
    replace: "  let id;\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
];
