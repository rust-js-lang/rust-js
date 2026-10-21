// Mutations of src/runtime/size_hint.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
  {
    name: "size-hint-not-adapters-own",
    breaks: "an explicit adapter's size hint isn't its own",
    file: "src/runtime/size_hint.js",
    find: "  if (it.hint !== undefined) return it.hint();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
  {
    name: "size-hint-not-what-it-steps",
    breaks: "a `$iterator`'s size hint isn't that of the JS iterator it steps",
    file: "src/runtime/size_hint.js",
    find: "  if (typeof it.of?.next === \"function\") return $sizeHint(it.of);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
];
