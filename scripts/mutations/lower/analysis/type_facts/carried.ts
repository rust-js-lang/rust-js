// Mutations of src/lower/analysis/type_facts/carried.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-associated-uncarried",
    breaks: "a fact of a library trait's associated type, `size_of::<A::Item>()` of smallvec's code a consumer compiles, is no dictionary's, so it's refused, \"`size_of` of a type parameter\" (#476)",
    file: "src/lower/analysis/type_facts/carried.rs",
    find: "        ) if def_id.is_local() || foreign.is_library(def_id.krate) => Some(*def_id),\n",
    replace: "        ) if def_id.is_local() => Some(*def_id),\n",
    tests: ["test/crates.test.ts", "-t", "item_size_of_instance"],
  },
  {
    name: "uncarried-associated-read",
    breaks: "a consumer's fact of a library trait's associated type that the library's dictionaries don't carry is read of them, `undefined` (#476)",
    file: "src/lower/analysis/type_facts/carried.rs",
    find: "    assoc.is_local() || foreign.carried(assoc).contains(&fact)\n",
    replace: "    assoc.is_local() || foreign.is_library(assoc.krate) || foreign.carried(assoc).contains(&fact)\n",
    tests: ["test/crates.test.ts", "-t", "associated type its dictionaries don't carry"],
  },
];
