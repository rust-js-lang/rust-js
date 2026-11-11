// Mutations of src/runtime/once_with.js (#1373).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-with-calls-f-where-made",
    breaks: "`iter::once_with(f)` calls `f` where it's made, not when it's first stepped (#1373)",
    file: "src/runtime/once_with.js",
    find: "function $onceWith(f) {\n  let held = true;\n",
    replace: "function $onceWith(f) {\n  const made = f();\n  f = () => made;\n  let held = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_once_with.rs$"],
  },
  {
    name: "once-with-calls-f-each-step",
    breaks: "`iter::once_with(f)` calls `f` for each step, so it never ends (#1373)",
    file: "src/runtime/once_with.js",
    find: "    held = false;\n    return { done: false, value: f() };\n",
    replace: "    return { done: false, value: f() };\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_once_with.rs$"],
  },
  {
    name: "once-with-hint-after-stepped",
    breaks: "`$onceWith`'s hint is `(1, Some(1))` after it's stepped, where std's `OnceWith`, holding no closure, hints `(0, Some(0))` (#1375)",
    file: "src/runtime/once_with.js",
    find: "  it.sizeHint = () => (held ? [1, 1] : [0, 0]);\n",
    replace: "  it.sizeHint = () => [1, 1];\n",
    tests: ["test/corpus.test.ts", "-t", "^once_and_once_with_hint.rs$"],
  },
];
