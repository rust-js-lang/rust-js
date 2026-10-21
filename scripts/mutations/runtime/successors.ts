// Mutations of src/runtime/successors.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "successors-behind",
    breaks: "`successors` finds the next item after it gives this one, calling its closure once less",
    file: "src/runtime/successors.js",
    find: "  it.next = () => {\n    if (next == null) return { done: true, value: undefined };\n    const item = boxed ? $someValue(next) : next;\n    next = successor(item);\n    return { done: false, value: item };\n  };",
    replace: "  let pending;\n  it.next = () => {\n    if (pending !== undefined) {\n      next = successor(pending);\n      pending = undefined;\n    }\n    if (next == null) return { done: true, value: undefined };\n    const item = boxed ? $someValue(next) : next;\n    pending = item;\n    return { done: false, value: item };\n  };",
    tests: ["test/corpus.test.ts","-t","iterator_sources"],
  },
  {
    name: "successors-held-not-dropped",
    breaks: "`successors` closed early never drops the item it holds",
    file: "src/runtime/successors.js",
    find: "      drop?.(held);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^successors_drops.rs$"],
  },
];
