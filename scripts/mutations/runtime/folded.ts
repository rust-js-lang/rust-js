// Mutations of src/runtime/folded.js (#162).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "folded-steps",
    breaks: "what std routes through `fold`, `last` and `collect` into a string or a hash map, steps the crate's iterator by `next`",
    file: "src/runtime/folded.js",
    find: "    (items ??= it\n      .fold([], (list, item) => {\n        list.push(item);\n        return list;\n      })\n      .values());\n",
    replace: "    (items ??= it.toArray().values());\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "folded-of-no-fold-throws",
    breaks: "$folded of a JS iterator with no fold is itself",
    file: "src/runtime/folded.js",
    find: "  if (it.fold === undefined) return it;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "folded-fold-unexposed",
    breaks: "what std routes through `fold` is listed before it's given on: `last()` of a `take(2).skip(0)` of the crate's iterator closes its source, dropping what it holds, before the item it replaces (#493)",
    file: "src/runtime/folded.js",
    find: "  folded.fold = (init, f, unwind) => (items === undefined ? it.fold(init, f, unwind) : listed().reduce(f, init));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_order_through_take_adapters.rs$"],
  },
];
