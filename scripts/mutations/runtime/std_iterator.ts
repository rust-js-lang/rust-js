// 
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-iterator-nth-unhooked",
    breaks: "a fuse's compiled `nth` isn't its JS iterator's, so `nth` steps by `next`",
    file: "src/runtime/std_iterator.js",
    find: "    it.nth = (k) => step(steps.nth(it, k));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_last_and_nth_passed_on.rs$"],
  },
  {
    name: "std-iterator-last-not-own",
    breaks: "a fuse's compiled `last` isn't said to be its own, so generic code's `last` of it steps by `next`",
    file: "src/runtime/std_iterator.js",
    find: "    it.last = () => step(steps.last(it));\n    it.ownLast = true;\n",
    replace: "    it.last = () => step(steps.last(it));\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_last_dropping.rs$"],
  },
];
