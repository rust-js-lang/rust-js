// Mutations of src/runtime/folded.js (#162).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "folded-steps",
    breaks: "what std routes through `fold`, `last` and `collect` into a string or a hash map, steps the crate's iterator by `next`",
    file: "src/runtime/folded.js",
    find: "    (items ??= foldOf([], (list, item) => {\n      list.push(item);\n      return list;\n    }).values());\n",
    replace: "    (items ??= it.toArray().values());\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "folded-of-no-fold-throws",
    breaks: "$folded of a JS iterator with no fold is itself",
    file: "src/runtime/folded.js",
    find: "  if (it.fold === undefined) return it;\n",
    replace: "",
    tests: ["test/lowering-iterators.test.ts", "-t", "a fold-taking collect in generic code of a JS iterator"],
  },
  {
    name: "folded-fold-unexposed",
    breaks: "what std routes through `fold` is listed before it's given on: a hash map collected of a `take(2).skip(0)` of the crate's iterator closes its source, dropping what it holds, before the value it replaces (#493, #494)",
    file: "src/runtime/folded.js",
    find: "  folded.fold = (init, f, unwind) => (items === undefined ? foldOf(init, f, unwind) : listed().reduce(f, init));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^skip_folded_by_consumers.rs$"],
  },
  {
    name: "folded-first-by-fold",
    breaks: "a `String` collected of `String`s folds its first item too, so a `skip`'s first step asks the crate's `nth(n - 1)`, where std takes it by `next`, `nth(n)` (#494)",
    file: "src/runtime/folded.js",
    find: "    if (!firstByNext) return it.fold(init, f, unwind);\n",
    replace: "    return it.fold(init, f, unwind);\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_folded_by_consumers.rs$"],
  },
];
