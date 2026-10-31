// Mutations of src/lower/body_queries/ranges/found.rs (rd316, ADR 0576).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "found-index-unbound",
    breaks:
      "the index a slice's `position` finds isn't known below its length, so `children.remove(index)` is `$removeAt(children, index)`, not `children.splice(index, 1)`",
    file: "src/lower/body_queries/ranges/found.rs",
    find: "            k.vars.insert(var, found);\n",
    replace: "            let _ = (var, found, k);\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
  {
    name: "found-index-at-most-the-length",
    breaks:
      "the index a slice's `position` finds is taken as at most its length, not below it, so `remove` of it is `$removeAt`",
    file: "src/lower/body_queries/ranges/found.rs",
    find: "        let below = Below::LENGTH.offset(-1);\n",
    replace: "        let below = Below::LENGTH;\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
];
