// Mutations of src/runtime/find_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "find-dropping-by-next",
    breaks: "a `find` of items with a destructor steps the iterator by `next`, not std's `try_fold`: std's `Filter::next` of a `skip()` asks its source `nth(2)` where Rust asks `nth(1)` (ADR 0932)",
    file: "src/runtime/find_dropping.js",
    find: "  if (it.tryFold !== undefined) {\n",
    replace: "  if (false) {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^skip_try_fold with"],
  },
  {
    name: "find-dropping-keeps-rejected",
    breaks: "`find` of items with a destructor drops none it rejects",
    file: "src/runtime/find_dropping.js",
    find: "    drop?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
];
