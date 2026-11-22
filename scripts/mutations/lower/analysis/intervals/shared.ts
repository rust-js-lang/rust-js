// Mutations of src/lower/analysis/intervals/shared.rs (#2310): a `State`'s
// maps and sets answer as `IndexMap`'s and `IndexSet`'s, and share what
// they hold.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shared-insert-again-last",
    breaks: "a key inserted again moves last, where an `IndexMap` keeps its place, so a walk of a state's facts, `join`'s or a lookup's first found, goes in another order",
    file: "src/lower/analysis/intervals/shared.rs",
    find: "            Some(&place) => self.entries.insert(place, (key, value)).map(|(_, old)| old),\n",
    replace:
      "            Some(&place) => {\n                let old = self.entries.remove(&place).map(|(_, old)| old);\n                self.places.insert(key.clone(), self.next);\n                self.entries.insert(self.next, (key, value));\n                self.next += 1;\n                old\n            }\n",
    unitTests: "lower::analysis::intervals::shared::tests::a_shared_map_answers_as_an_index_map_does",
  },
  {
    name: "shared-equal-in-order",
    breaks: "two maps are equal only with their entries in the same order, where `IndexMap`s are in any, so the walk sees a block's entry change where it didn't and visits it again, widening sooner",
    file: "src/lower/analysis/intervals/shared.rs",
    find: "            || (self.len() == other.len() && self.iter().all(|(key, value)| other.get(key) == Some(value)))\n",
    replace: "            || (self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b))\n",
    unitTests: "lower::analysis::intervals::shared::tests::a_shared_map_answers_as_an_index_map_does",
  },
  {
    name: "shared-union-other-first",
    breaks: "a set's union walks the other's items first, where `IndexSet::union` walks its own, so a join's `lent_away` and `escaped` go in another order",
    file: "src/lower/analysis/intervals/shared.rs",
    find: "        self.iter().chain(other.iter().filter(|item| !self.contains(item)))\n",
    replace: "        other.iter().chain(self.iter().filter(|item| !other.contains(item)))\n",
    unitTests: "lower::analysis::intervals::shared::tests::a_shared_set_answers_as_an_index_set_does",
  },
  {
    name: "shared-insert-unshared",
    breaks: "a new key's insert copies the whole map, sharing nothing with the states it was cloned from, so the walk of a function of 3,000 calls holds every block's facts whole again, gigabytes (#2310)",
    file: "src/lower/analysis/intervals/shared.rs",
    find: "                self.next += 1;\n                None\n",
    replace: "                self.next += 1;\n                self.entries = self.entries.iter().map(|(place, entry)| (*place, entry.clone())).collect();\n                None\n",
    tests: ["test/lowering-functions.test.ts", "-t", "a long function is lowered block by block"],
  },
];
