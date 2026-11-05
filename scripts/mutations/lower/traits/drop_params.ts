// Mutations of src/lower/traits/drop_params.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-params-never-drop",
    breaks:
      "a library with no destructor of its own takes its consumers' types as dropping nothing, so a value of an associated type it holds is never dropped (ADRs 0163, 0844)",
    file: "src/lower/traits/drop_params.rs",
    find: "    library || drop_trait.is_some_and(",
    replace: "    drop_trait.is_some_and(",
    tests: ["test/crates.test.ts", "-t", "two crates: rc_per_type"],
  },
  {
    name: "copy-param-given-a-drop",
    breaks:
      "a `Copy` type parameter is given a drop function too, which its callers pass for nothing",
    file: "src/lower/traits/drop_params.rs",
    find: "        && !tcx.type_is_copy_modulo_regions(typing_env, Ty::new_param(tcx, param.index, param.name))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copy_param_no_drop.rs$"],
    snapshots: true,
  },
  {
    name: "trait-method-item-drops-undeclared",
    breaks:
      "a trait's generic method takes no drop for a std trait's associated type of its own type parameter, so a dictionary's entry can't give one",
    file: "src/lower/traits/drop_params.rs",
    find: "        indices.extend(place.map(|place| count + place as u32));\n",
    replace: "        let _ = (place, count);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_method_drops.rs$"],
  },
  {
    name: "item-drops-of-named-items",
    breaks:
      "an associated type a bound names, `IntoIterator<Item = T>`, takes a drop of its own",
    file: "src/lower/traits/drop_params.rs",
    find: "                if !named && !items.iter().any(|&t| alias_of(t) == alias_of(projection)) {\n",
    replace:
      "                if !items.iter().any(|&t| alias_of(t) == alias_of(projection)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
    snapshots: true,
  },
  {
    name: "item-drops-of-dictionary-traits",
    breaks:
      "an associated type of a trait whose dictionary has its drop takes a drop of its own too, `dropZOffset?.(offset)` where its dictionary's `ZZone.$dropOffset?.(offset)` drops it",
    file: "src/lower/traits/drop_params.rs",
    find: "        if !slotted || has_dictionary_of(tcx, foreign, bound.def_id(), self_ty) {\n",
    replace: "        if !slotted {\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_drops.rs$"],
    snapshots: true,
  },
  {
    name: "nested-item-unslotted",
    breaks:
      "an associated type of one a bound names, `<I::Item as IntoIterator>::Item`, has no drop slot, and its values are refused",
    file: "src/lower/traits/drop_params.rs",
    find: "            || (alias_of(self_ty).is_some() && items.iter().any(|&t| alias_of(t) == alias_of(self_ty)));\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_item_drops.rs$"],
  },
  {
    name: "built-layout-gate-inverted",
    breaks:
      "a caller gives a built slot's layout only where each of its gates' sources is never collected in place, so a collect that is in place reads `undefined`, and a panic drops what's left first where Rust drops what it collected (#737)",
    file: "src/lower/traits/drop_params.rs",
    find: "            || built.gates.iter().any(|&gate| {\n",
    replace: "            || !built.gates.iter().any(|&gate| {\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type.rs$"],
  },
  {
    name: "built-slot-misplaced",
    breaks: "a built slot's index names the next one of its function's, or none (#737)",
    file: "src/lower/traits/drop_params.rs",
    find: "        Some(slots) => slots.get(at).cloned(),\n",
    replace: "        Some(slots) => slots.get(at + 1).cloned(),\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type.rs$"],
  },
  {
    name: "built-tuple-unnamed",
    breaks: "a built tuple's layout is named as rustc prints it, `(usize, I::Item)Size`, which is no JS name (#737)",
    file: "src/lower/traits/drop_params.rs",
    find: "        ty::Tuple(parts) => (parts.iter())\n",
    replace: "        ty::Tuple(parts) if false => (parts.iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
    snapshots: true,
  },
];
