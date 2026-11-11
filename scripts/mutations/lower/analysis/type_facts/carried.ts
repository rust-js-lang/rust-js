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
  {
    name: "into-iterator-sizes-uncarried",
    breaks: "a library's trait's associated `IntoIterator`'s constant size isn't carried whatever asks it, so a consumer's generic flatten of its own array impl's items is refused, \"which no caller gives\" (#1341)",
    file: "src/lower/analysis/type_facts/carried.rs",
    find: "            if crate::lower::recognition::trait_holds_in(tcx, typing_env, projection, into_iterator) == Some(true)\n",
    replace: "            if crate::lower::recognition::trait_holds_in(tcx, typing_env, projection, into_iterator) == Some(false)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_trait_items_flattened_where_its_hint_is_seen"],
  },
  {
    name: "iterator-assoc-sizes-carried",
    breaks: "an associated type its bounds make an `Iterator` has its constant size carried by every impl's dictionary, so an impl that makes it its `T: Iterator` asks it, and is refused (#1341)",
    file: "src/lower/analysis/type_facts/carried.rs",
    find: "                && crate::lower::recognition::const_size_unknown(tcx, typing_env, projection)\n",
    replace: "                && true\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_passed_through_a_chain_of_impls.rs$"],
  },
];
