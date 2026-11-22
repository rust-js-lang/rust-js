// The iterator half of MIR's iterator calls (src/lower/iterators/mir_steps.rs,
// #2299): its own items, the adapters the recognizers make in place.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rev-in-place-compiled",
    breaks:
      "a `rev()` stepped where it's made is std's compiled `rev()`, not the recognizers'",
    file: "src/lower/iterators/mir_steps.rs",
    find: "        self.frame.facts.in_place_adapters.made_at(self.frame.thir, span)\n",
    replace: "        self.frame.facts.in_place_adapters.made_at(self.frame.thir, span) && false\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
];
