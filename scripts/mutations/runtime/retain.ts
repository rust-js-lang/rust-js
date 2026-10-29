// Mutations of src/runtime/retain.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "retain-drops-none",
    breaks: "`retain` of items with a destructor drops none it doesn't keep",
    file: "src/runtime/retain.js",
    find: "        drop?.(x);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
  {
    name: "retain-left-as-thrown",
    breaks: "a `retain` whose drop or test panics leaves the item it dropped in the vector, or loses what it hadn't tested, so the vector's own drop drops one twice or one never (#416)",
    file: "src/runtime/retain.js",
    find: "    v.length = n + rest;\n",
    replace: "    v.length = n;\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
];
