// Mutations of src/runtime/count.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "count-field-called",
    breaks: "`count()` of an `enumerate()` given to generic code calls its `count`, how many it's given: `TypeError: it.count is not a function` (ADR 0988)",
    file: "src/runtime/count.js",
    find: "  if (typeof it.count === \"function\") return it.count(drop);\n",
    replace: "  if (it.count !== undefined) return it.count(drop);\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_counted_by_generic_code.rs$"],
  },
  {
    name: "enumerate-count-by-fold",
    breaks: "`count()` of an `enumerate()` counts each it gives, not its source's `count`, so a reversed `Vec`'s take drops back to front (#493)",
    file: "src/runtime/count.js",
    find: "  if (it.countItems !== undefined) return it.countItems(drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_order_through_take_adapters.rs$"],
  },
];
