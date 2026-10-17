// Mutations of src/runtime/retain.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "retain-drops-none",
    breaks: "`retain` of items with a destructor drops none it doesn't keep",
    file: "src/runtime/retain.js",
    find: "    } else if (drop !== undefined) {\n      drop(x);\n    }\n",
    replace: "    }\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
];
