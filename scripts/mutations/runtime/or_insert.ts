// Mutations of src/runtime/or_insert.js (ADR 0321).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "or-insert-keeps-given",
    breaks: "`or_insert` of a value with a destructor where the key's there drops none of the one given",
    file: "src/runtime/or_insert.js",
    find: "  } else if (drop !== undefined) {\n    drop(value);\n  }\n",
    replace: "  }\n",
    tests: ["test/corpus.test.ts", "-t", "^map_value_destructors.rs$"],
  },
];
