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
  {
    name: "iter-of-iter-rewrapped",
    breaks: "an iterator a variable holds, given on, is wrapped again, a `$iter` of it, so it steps nothing",
    file: "src/runtime/iter.js",
    find: "  if (!Array.isArray(items) && items.back !== undefined) return items;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "iter-len-whole",
    breaks: "a `$iter`'s `len()` is its whole array's, not what's left",
    file: "src/runtime/iter.js",
    find: "    return this.end - this.at;\n",
    replace: "    return this.items.length;\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "lowerless-iter-hint-counted",
    breaks: "`$iter(items, true)`, of what std hints no lower bound of, hints what's left as its lower bound, so a crate that reads a capacity grows a collect of a B-tree's `range(..)` otherwise (#1215)",
    file: "src/runtime/iter.js",
    find: "      return [0, this.end - this.at];\n",
    replace: "      return [this.end - this.at, this.end - this.at];\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
  },
  {
    name: "iter-of-what-knows-no-back-quiet",
    breaks: "`$iter` steps an iterator that doesn't know its back as an empty array, a silent `[]`, where it throws (#1292)",
    file: "src/runtime/iter.js",
    find: "  if (typeof items.length !== \"number\") throw new TypeError(\"rust-js: an iterator stepped as an array's, which doesn't know its back\");\n",
    replace: "",
    tests: ["test/lowering-generic-iterators.test.ts", "-t", "never stepped as an empty array"],
  },
];
