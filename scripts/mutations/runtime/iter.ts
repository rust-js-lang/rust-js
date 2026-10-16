// Mutations of src/runtime/iter.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iter-end-ignored",
    breaks: "a stepped iterator gives what `next_back()` took",
    file: "src/runtime/iter.js",
    find: "this.at < this.end ? { value: this.items[this.at++]",
    replace: "this.at < this.items.length ? { value: this.items[this.at++]",
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
  {
    name: "next-back-from-front",
    breaks: "`next_back()` takes the first item",
    file: "src/runtime/iter.js",
    find: "{ value: this.items[--this.end], done: false }",
    replace: "{ value: this.items[this.at++], done: false }",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
  {
    name: "next-back-past-front",
    breaks: "`next_back()` takes items `next()` took",
    file: "src/runtime/iter.js",
    find: "    return this.at < this.end ? { value: this.items[--this.end]",
    replace: "    return this.end > 0 ? { value: this.items[--this.end]",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
