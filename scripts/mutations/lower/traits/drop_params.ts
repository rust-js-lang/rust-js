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
  {
    name: "std-method-facts-none",
    breaks:
      "a std trait's required generic method carries nothing of its own type parameters, so `impl FromIterator<D> for Req`'s `Vec::from_iter(iter)` asks its `I::IntoIter`'s in-place buffer of no caller, refused (#794)",
    file: "src/lower/traits/drop_params.rs",
    find: "    if method.is_local() || foreign.is_library(method.krate) || tcx.trait_of_assoc(method).is_none() {\n",
    replace: "    if true || method.is_local() || foreign.is_library(method.krate) || tcx.trait_of_assoc(method).is_none() {\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_impl_of_dropping.rs$"],
  },
  {
    name: "std-method-facts-of-provided",
    breaks:
      "a std trait's provided method, whose std body the crate compiles and asks its own (ADR 0561), carries its declaration's facts too: `Iterator::collect` given `B: FromIterator` asks an `InPlace` its body never reads, the JS of every generic collect another parameter (#794)",
    file: "src/lower/traits/drop_params.rs",
    find: "    if tcx.associated_item(method).defaultness(tcx).has_value() {\n",
    replace: "    if false && tcx.associated_item(method).defaultness(tcx).has_value() {\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_chain_of_into|into_iter_arguments|zip_into_mut_slice|backs_given_behind_impls|adapter_drops|held_adapters_stepped_to_their_end).rs$"],
    snapshots: true,
  },
  {
    name: "default-where-clause-unslotted",
    breaks:
      "an associated type only a copied default's own where clause names, `<Self::Item as IntoIterator>::IntoIter`, is no slot of its impl's, so the `iter` of a `Held` it pushes is never dropped (#798)",
    file: "src/lower/traits/drop_params.rs",
    find: "        bounds.push((clauses, ty::TypingEnv::new(param_env, typing_env.typing_mode())));\n",
    replace: "        let _ = (clauses, param_env);\n",
    tests: ["test/corpus.test.ts", "-t", "^default_where_clause_item_drops.rs$"],
  },
  {
    name: "default-where-clause-unnormalized",
    breaks:
      "a copied default's where clause of a non-blanket impl, `<Wrapper<I> as Iterator>::Item: IntoIterator`, isn't in the impl's terms as its body is, `I::Item`, so its `Held`'s `iter` is never dropped (#803)",
    file: "src/lower/traits/drop_params.rs",
    find: "                (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(clause))).unwrap_or(clause)\n",
    replace: "                clause\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_of_a_non_blanket_impl_steps_its_self).rs$"],
  },
];
