// Mutations of src/runtime/dropping.js (ADR 0484).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dropping-carries-nothing",
    breaks: "`$dropping` gives back the closure carrying no drop",
    file: "src/runtime/dropping.js",
    find: "  f[$disposeKey] = drop;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "dispose-plain-function",
    breaks: "a `dyn Fn` of a function that carries no drop throws as it's dropped",
    file: "src/runtime/dropping.js",
    find: "  const drop = f[$disposeKey];\n  if (drop === undefined) return;\n",
    replace: "  const drop = f[$disposeKey];\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "dropping-once-not-on-unwind",
    breaks: "a `Fn` closure made a `dyn FnOnce` whose call panics never drops what it took as it unwinds (#409)",
    file: "src/runtime/dropping.js",
    find: "    try {\n      return f(...args);\n    } finally {\n      drop();\n    }\n",
    replace: "    const result = f(...args);\n    drop();\n    return result;\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_boxed_once_drops.rs$"],
  },
  {
    name: "borrowed-is-the-function",
    breaks: "`$borrowed(f)` gives back `f` itself, and a `dyn Fn` of a borrowed function runs the drop `f` carries as it's dropped",
    file: "src/runtime/dropping.js",
    find: "  return (...args) => f(...args);\n",
    replace: "  return f;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_of_a_borrowed_function.rs$"],
  },
  {
    name: "dispose-twice",
    breaks: "a closure that carries its drop is dropped again by its caller after the fold it was moved into dropped it (#710)",
    file: "src/runtime/dropping.js",
    find: "  f[$disposeKey] = undefined;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
];
