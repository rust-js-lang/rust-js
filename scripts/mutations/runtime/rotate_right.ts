// Mutations of src/runtime/rotate_right.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rotate-right-leftwards",
    breaks: "`rotate_right` rotates left",
    file: "src/runtime/rotate_right.js",
    find: "    v[(i + n) % items.length] = items[i];",
    replace: "    v[i] = items[(i + n) % items.length];",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
];
