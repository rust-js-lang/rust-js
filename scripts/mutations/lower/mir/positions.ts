// Mutations of src/lower/mir/positions.rs (rd316, ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "found-index-not-fresh",
    breaks:
      "a slice's iterator stepped before, `it.next(); it.position(f)`, is searched as its array, so the index found counts from its first item, not from where it was",
    file: "src/lower/mir/positions.rs",
    find: "        made.filter(|_| uses.1 == 2)\n",
    replace: "        made\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
  },
  {
    name: "found-index-escaping",
    breaks:
      "a `position`'s `Option` given back is `indexOf`'s, `-1` where it's `None`, which its caller reads as `Some(-1)`",
    file: "src/lower/mir/positions.rs",
    find: "                && tested_and_taken(body, destination.local, 1)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
  },
  {
    name: "found-index-never-index-of",
    breaks:
      "a `position` by `===` is `children.findIndex((c) => c === child)`, not `children.indexOf(child)` as a person writes it",
    file: "src/lower/mir/positions.rs",
    find: "            Some(x) => Expr::call(Expr::member(items, \"indexOf\"), vec![x]),\n",
    replace: "            Some(_) => Expr::call(Expr::member(items, \"findIndex\"), vec![f]),\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
];
