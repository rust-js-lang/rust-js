// Mutations of src/lower/body_queries/ranges/volatile.rs (ADR 0576).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "const-pointer-of-shared-volatile",
    breaks:
      "a `*const` of what a shared reference refers to makes its variable volatile, so `ptr::eq(&node.children[0], skip)` leaves `node.children[diffIdx]` below its length `$index`",
    file: "src/lower/body_queries/ranges/volatile.rs",
    find: "            } if through_shared(thir, arg) => None,\n",
    replace: "            } if false => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_items_and_counts.rs$"],
    snapshots: true,
  },
];
