// Mutations of src/runtime/iter.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iter-end-ignored",
    breaks: "a stepped iterator gives what `next_back()` took",
    file: "src/runtime/iter.js",
    find: "this.at < this.end ?",
    replace: "this.at < this.items.length ?",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
  {
    name: "iter-not-an-iterator",
    breaks: "a `$iter` has none of `Iterator`'s helpers, nor is it iterable",
    file: "src/runtime/iter.js",
    find: "  const it = Object.create(Iterator.prototype);\n",
    replace: "  const it = {};\n",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
