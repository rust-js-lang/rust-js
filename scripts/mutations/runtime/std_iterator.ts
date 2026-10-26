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
  {
    name: "std-iterator-back-own-unknown",
    breaks: "a compiled `fuse()` isn't given its `rfold` and `nth_back` where they're asked (#224)",
    file: "src/runtime/std_iterator.js",
    find: "    if (rfold !== undefined) it.rfold = (init, f) => rfold(it, init, f);\n    if (nthBack !== undefined) it.nthBack = (k) => step(nthBack(it, k));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "std-iterator-close-unrun",
    breaks: "closing a compiled `Rev` drops nothing it holds",
    file: "src/runtime/std_iterator.js",
    find: "      steps.close(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "std-iterator-takes-unmarked",
    breaks: "std's struct its compiled `fold` took isn't marked closed, so its close drops what it held again (ADR 0912)",
    file: "src/runtime/std_iterator.js",
    find: "    if (steps.takes?.includes(method)) it.$closed = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
];
