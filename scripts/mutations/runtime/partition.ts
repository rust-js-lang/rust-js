// Mutations of src/runtime/partition.js (#162, #179).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "partition-listed-first",
    breaks: "`partition` lists every item before its predicate is given the first",
    file: "src/runtime/partition.js",
    find: "    it.forEach((item) => {\n",
    replace: "    it.toArray().forEach((item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^consumers_step_each.rs$"],
  },
  {
    name: "partition-steps",
    breaks: "`partition` of the crate's iterator steps it by `next`, not by its own `fold`",
    file: "src/runtime/partition.js",
    find: "    it.forEach((item) => {\n",
    replace: "    Iterator.prototype.forEach.call(it, (item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "partition-throw-keeps-sides",
    breaks: "a `partition` whose predicate panics leaves what it had put on each side undropped (#423)",
    file: "src/runtime/partition.js",
    find: "      try {\n        $dropItems(no, drop);\n      } finally {\n        $dropItems(yes, drop);\n      }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
];
