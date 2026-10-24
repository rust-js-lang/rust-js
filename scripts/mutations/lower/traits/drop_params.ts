// Mutations of src/lower/traits/drop_params.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-params-never-drop",
    breaks: "a library with no destructor of its own takes its consumers' types as dropping nothing, so a value of an associated type it holds is never dropped (ADRs 0163, 0844)",
    file: "src/lower/traits/drop_params.rs",
    find: "    library || drop_trait.is_some_and(",
    replace: "    drop_trait.is_some_and(",
    tests: ["test/crates.test.ts", "-t", "two crates: assoc_drop"],
  },
  {
    name: "copy-param-given-a-drop",
    breaks: "a `Copy` type parameter is given a drop function too, which its callers pass for nothing",
    file: "src/lower/traits/drop_params.rs",
    find: "        && !tcx.type_is_copy_modulo_regions(typing_env, Ty::new_param(tcx, param.index, param.name))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copy_param_no_drop.rs$"],
    snapshots: true,
  },
  {
    name: "trait-method-item-drops-undeclared",
    breaks: "a trait's generic method takes no drop for a std trait's associated type of its own type parameter, so a dictionary's entry can't give one",
    file: "src/lower/traits/drop_params.rs",
    find: "        indices.extend(place.map(|place| count + place as u32));\n",
    replace: "        let _ = (place, count);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_method_drops.rs$"],
  },
  {
    name: "item-drops-of-named-items",
    breaks: "an associated type a bound names, `IntoIterator<Item = T>`, takes a drop of its own",
    file: "src/lower/traits/drop_params.rs",
    find: "                if !named && !items.iter().any(|&t| alias_of(t) == alias_of(projection)) {\n",
    replace: "                if !items.iter().any(|&t| alias_of(t) == alias_of(projection)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
    snapshots: true,
  },
  {
    name: "item-drops-of-dictionary-traits",
    breaks: "an associated type of a trait whose dictionary has its drop takes a drop of its own too",
    file: "src/lower/traits/drop_params.rs",
    find: "        if !slotted || has_dictionary_of(tcx, foreign, bound.def_id(), self_ty) {\n",
    replace: "        if !slotted {\n",
    tests: ["test/crates.test.ts", "-t", "consumer's iterator"],
  },
  {
    name: "nested-item-unslotted",
    breaks: "an associated type of one a bound names, `<I::Item as IntoIterator>::Item`, has no drop slot, and its values are refused",
    file: "src/lower/traits/drop_params.rs",
    find: "            || (alias_of(self_ty).is_some() && items.iter().any(|&t| alias_of(t) == alias_of(self_ty)));\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_item_drops.rs$"],
  },
];
