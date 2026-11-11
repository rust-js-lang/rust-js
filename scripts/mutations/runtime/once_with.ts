// Mutations of src/runtime/once_with.js (#1373).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-with-calls-f-where-made",
    breaks: "`iter::once_with(f)` calls `f` where it's made, not when it's first stepped (#1373)",
    file: "src/runtime/once_with.js",
    find: "function* $onceWith(f) {\n  yield f();\n}",
    replace: "function $onceWith(f) {\n  return [f()][Symbol.iterator]();\n}",
    tests: ["test/corpus.test.ts", "-t", "^iter_once_with.rs$"],
  },
  {
    name: "once-with-calls-f-each-step",
    breaks: "`iter::once_with(f)` calls `f` for each step, so it never ends (#1373)",
    file: "src/runtime/once_with.js",
    find: "function* $onceWith(f) {\n  yield f();\n}",
    replace: "function* $onceWith(f) {\n  while (true) yield f();\n}",
    tests: ["test/corpus.test.ts", "-t", "^iter_once_with.rs$"],
  },
];
