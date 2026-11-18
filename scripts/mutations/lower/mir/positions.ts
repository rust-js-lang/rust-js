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
    find: "                && let Some(items) = fresh()\n                && destination.projection.is_empty()\n                && tested_and_taken(body, destination.local, 1)\n",
    replace: "                && let Some(items) = fresh()\n                && destination.projection.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
  },
  {
    name: "found-index-never-index-of",
    breaks:
      "a `position` by `===` is `children.findIndex((c) => c === child)`, not `children.indexOf(child)` as a person writes it",
    file: "src/lower/mir/positions.rs",
    find: "            Some(x) => Expr::builtin(items, \"indexOf\", vec![x]),\n",
    replace: "            Some(_) => Expr::call(Expr::member(items, \"findIndex\"), vec![f]),\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
  {
    name: "and-then-position-not-found",
    breaks: "an `and_then` whose closure gives a slice's `position`, its `Option` only tested, stays `ext != null ? $position(DAYS, (d) => d === ext) : undefined`, not JS's own search, `ext != null ? DAYS.indexOf(ext) : -1`, as the vercel-ui SDK reads (#2074)",
    file: "src/lower/mir/positions.rs",
    find: "                && self.gives_a_position(closure)\n",
    replace: "                && false && self.gives_a_position(closure)\n",
    tests: ["test/corpus.test.ts", "-t", "^position_through_and_then\\.rs$"],
    snapshots: true,
  },
  {
    name: "and-then-position-escaping-found",
    breaks: "an `and_then` of a slice's `position` whose `Option` leaves the function is taken for a found index, so its `None` is `-1` where its caller reads `undefined` (#2074)",
    file: "src/lower/mir/positions.rs",
    find: "            if let Some(closure) = then_closure\n                && self.gives_a_position(closure)\n                && destination.projection.is_empty()\n                && tested_and_taken(body, destination.local, 1)\n",
    replace: "            if let Some(closure) = then_closure\n                && self.gives_a_position(closure)\n                && destination.projection.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^position_through_and_then\\.rs$"],
  },
  {
    name: "and-then-position-of-any-iterator",
    breaks: "an `and_then`'s closure whose `position` is of an iterator that isn't a slice's `iter()` made for it is taken for one, its `$position` made an array's search of what isn't the array (#2074)",
    file: "src/lower/mir/positions.rs",
    find: "                        .is_some_and(|receiver| self.fresh_slice_items(body, receiver.local).is_some()))\n",
    replace: "                        .is_some_and(|_| true))\n",
    tests: ["test/corpus.test.ts", "-t", "^position_through_and_then\\.rs$"],
    snapshots: true,
  },
];
