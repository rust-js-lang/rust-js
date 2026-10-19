// Mutations of src/runtime/lazy_zip.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "zip-pulled-undropped",
    breaks: "an item `zip` steps where the other side's run out isn't dropped",
    file: "src/runtime/lazy_zip.js",
    find: "        dropA?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "zip-other-unclosed",
    breaks: "`zip`'s other side isn't closed with it, so what it hasn't given isn't dropped",
    file: "src/runtime/lazy_zip.js",
    find: "    right.return?.();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
];
