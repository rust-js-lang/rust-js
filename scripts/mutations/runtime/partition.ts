// Mutations of src/runtime/partition.js (#162, #179).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "partition-listed-first",
    breaks: "`partition` lists every item before its predicate is given the first",
    file: "src/runtime/partition.js",
    find: "  Iterator.from(items).forEach((item) => {\n",
    replace: "  Iterator.from(items).toArray().forEach((item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^consumers_step_each.rs$"],
  },
  {
    name: "partition-steps",
    breaks: "`partition` of the crate's iterator steps it by `next`, not by its own `fold`",
    file: "src/runtime/partition.js",
    find: "  Iterator.from(items).forEach((item) => {\n",
    replace: "  Iterator.prototype.forEach.call(Iterator.from(items), (item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
];
