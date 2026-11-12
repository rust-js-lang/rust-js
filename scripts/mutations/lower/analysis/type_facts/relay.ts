// Mutations of src/lower/analysis/type_facts/relay.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-fact-of-alias-not-asked",
    breaks:
      "a library function's fact of a std associated type of a caller's parameter, `I::IntoIter`'s `IFused`, isn't asked",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "    if let Some(slot) = slot_of(tcx, foreign, caller, of) {\n",
    replace: "    if let Some(slot) = slot_of(tcx, foreign, caller, of).filter(|_| matches!(of.kind(), ty::Param(_))) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_fuse.rs$"],
  },
  {
    name: "built-layout-unasked",
    breaks:
      "a collect's items of a type built of its function's type parameters, `Held<<H::Item as IntoIterator>::IntoIter>`, ask no caller their layout, and the collect is refused, `size_of` of a type parameter, as itertools' `multi_cartesian_product` was (#737)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "    ) && tcx.layout_of(typing_env.as_query_input(ty)).is_err()\n",
    replace: "    ) && tcx.layout_of(typing_env.as_query_input(ty)).is_err()\n        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type.rs$"],
  },
  {
    name: "built-layout-asked-though-never-read",
    breaks:
      "a generic caller giving a built slot's every gate a source std never collects in place still asks the layout of its own caller, which no collect reads (#737)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "                (false, true) => return Vec::new(),\n",
    replace: "                (false, true) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
    snapshots: true,
  },
  {
    name: "built-layout-unrelayed",
    breaks:
      "a generic caller of a function asking a built type's layout, of a type built of its own slots, `Held<IntoIter<T>>`, asks its own caller none, and its call is refused (#737)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "        .filter_map(|each| asked_of(tcx, foreign, typing_env, caller, each, &gates))\n",
    replace:
      "        .filter_map(|(of, fact)| Some(Asked::Slot(slot_of(tcx, foreign, caller, of)?, fact)).filter(|_| !gates.is_some()))\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "built-slot-sort-unasked",
    breaks: "generic code's `sort_by_cached_key` of a key that holds a type parameter is refused, its `(K, usize)` pairs' sort asked of no caller (#925)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "        TypeFact::Size | TypeFact::Room | TypeFact::Align | TypeFact::Sort | TypeFact::StableSort\n",
    replace: "        TypeFact::Size | TypeFact::Room | TypeFact::Align | TypeFact::StableSort\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_stable_seen.rs$"],
  },
  {
    name: "built-slot-stable-sort-unasked",
    breaks: "generic code's collect into a `BTreeMap<K, V>` of a type parameter's keys is refused, its `(K, V)` entries' stable sort asked of no caller (#926)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: " | TypeFact::Sort | TypeFact::StableSort\n",
    replace: " | TypeFact::Sort\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_generic.rs$|^generic_collect_grows_by_its_items_size.rs$"],
  },
  {
    name: "callee-fact-of-assoc-uncarried",
    breaks: "a fact a std callee asks of an associated type no slot names, a flatten's items' constant size of `<S as Source>::Items`, isn't carried by its trait's dictionaries, so it's refused, \"which no caller gives\" (#1312, #1341)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "        return Some(Asked::Carried(assoc, fact));\n",
    replace: "        return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_a_traits_items_sized_by_its_dictionary.rs$"],
  },
  {
    name: "built-trait-fact-unrelayed",
    breaks: "a fact of a std trait std's zip is specialized on, of a type built of a slot, `vec::IntoIter<<I as Iterator>::Item>`'s `TrustedRandomAccessNoCoerce`, is asked of no caller, so a library's zip its consumer compiles is refused (#1389)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "            crate::lower::recognition::trait_holds_in(tcx, typing_env, ty, fact_trait).is_none()\n",
    replace: "            false && crate::lower::recognition::trait_holds_in(tcx, typing_env, ty, fact_trait).is_none()\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_zip_of_a_vec_of_a_slots_items"],
  },
  {
    name: "nested-layouts-unbounded",
    breaks: "an impl whose associated type nests its own type around its parameter's asks its makers a layout one wider each round, forever (#1430)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "const NESTED_AT_MOST: usize = 8;\n",
    replace: "const NESTED_AT_MOST: usize = usize::MAX;\n",
    tests: ["test/diagnostics.test.ts", "-t", "nests in itself without end"],
  },
  {
    name: "nested-projections-unbounded",
    breaks: "a projection nested in itself, `<<T as Shape>::Uninit as Shape>::Uninit`, isn't counted, so its layouts are asked one wider each round, forever (#1430)",
    file: "src/lower/analysis/type_facts/relay.rs",
    find: "            ty::Projection { def_id } => Some((def_id, alias.args)),\n",
    replace: "            ty::Projection { def_id: _ } => None,\n",
    tests: ["test/diagnostics.test.ts", "-t", "nests in itself without end"],
  },
];
