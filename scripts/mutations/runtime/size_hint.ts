// Mutations of src/runtime/size_hint.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-size-hint-array-unknown",
    breaks: "`size_hint()` of a generic iterator that's an array is `(0, None)`, not its length",
    file: "src/runtime/size_hint.js",
    find: "  const left = Array.isArray(it) ? it.length : Array.isArray(it.items) ? it.end - it.at : undefined;",
    replace: "  const left = Array.isArray(it.items) ? it.end - it.at : undefined;",
    tests: ["test/corpus.test.ts", "-t", "size_hint"],
  },
  {
    name: "generic-size-hint-stepped-whole",
    breaks: "`size_hint()` of a generic iterator partly stepped counts the items it's past",
    file: "src/runtime/size_hint.js",
    find: "Array.isArray(it.items) ? it.end - it.at : undefined;",
    replace: "Array.isArray(it.items) ? it.items.length : undefined;",
    tests: ["test/corpus.test.ts", "-t", "size_hint"],
  },
  {
    name: "size-hint-past-end",
    breaks: "a generic `size_hint()` counts what `next_back()` took",
    file: "src/runtime/size_hint.js",
    find: "it.end - it.at",
    replace: "it.items.length - it.at",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
