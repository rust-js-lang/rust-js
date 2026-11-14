// Mutations of src/lower/traits/drop_params.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dictionary-supertrait-items-unslotted",
    breaks:
      "a function bounded by the crate's trait that extends `Iterator`, `T: Sorted`, has no slot of `<T as Iterator>::Item`, so it can't give `sorted` its sort's facts: refused (#934)",
    file: "src/lower/traits/drop_params.rs",
    find: "            if dictionary && (tr == bound.def_id() || has_dictionary_of(tcx, foreign, tr, self_ty)) {\n",
    replace: "            if dictionary {\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_sorted.rs$"],
  },
  {
    name: "item-drop-of-made-iterator-itself",
    breaks:
      "an associated type's drop, of the crate's iterator generic code has as the JS iterator its dictionary made, drops that JS iterator as the struct: `drop at undefined` (#978)",
    file: "src/lower/traits/drop_params.rs",
    find: "            let iterated = !as_it_is && self.item_made_js_iterator(item, item_ty);\n",
    replace: "            let iterated = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_dropped.rs$"],
  },
  {
    name: "library-params-never-drop",
    breaks:
      "a library with no destructor of its own takes its consumers' types as dropping nothing, so a value of an associated type it holds is never dropped (ADRs 0163, 0844)",
    file: "src/lower/traits/drop_params.rs",
    find: "    library || drop_trait.is_some_and(",
    replace: "    drop_trait.is_some_and(",
    tests: ["test/crates.test.ts", "-t", "two crates: (rc_per_type|library_without_destructors_drops_what_a_pointer_holds)"],
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
    find: "        indices.extend(place.map(|place| item_slot(tcx, method, place)));\n",
    replace: "        let _ = place;\n",
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
    find: "        let dictionary = has_dictionary_of(tcx, foreign, bound.def_id(), self_ty);\n",
    replace: "        let dictionary = false;\n",
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
    name: "unmet-slot-fact-given",
    breaks:
      "a caller gives an impl's slot facts of an associated type no value is there, of a default whose where clause it can't meet, and is refused its `size_of` (#828)",
    file: "src/lower/traits/drop_params.rs",
    find: "        !crate::lower::drops::no_type_here(self.tcx, self.typing_env, self.drop_slot_of(id, index, args))\n",
    replace: "        true || !crate::lower::drops::no_type_here(self.tcx, self.typing_env, self.drop_slot_of(id, index, args))\n",
    tests: ["test/corpus.test.ts", "-t", "^blanket_default_beside_an_unmet_clause.rs$"],
  },
  {
    name: "slot-facts-unasked",
    breaks: "a map keyed by `Self::Item` asks no fact of it",
    file: "src/lower/traits/drop_params.rs",
    find: "        ty::Alias(..) => {\n            let alias = alias_of(ty)?;\n",
    replace: "        ty::Alias(..) if false => {\n            let alias = alias_of(ty)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_maps.rs$"],
  },
  {
    name: "slot-of-another-functions-param",
    breaks:
      "a type parameter not of the function, a copied default's own numbered past the impl's, is taken for the impl's slot of its number (#804)",
    file: "src/lower/traits/drop_params.rs",
    find: "        ty::Param(param) => (slot_item(tcx, id, param.index).is_none()).then_some(param.index),\n",
    replace: "        ty::Param(param) => Some(param.index),\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
    snapshots: true,
  },
  {
    name: "impl-written-clauses-unslotted",
    breaks: "a method an impl writes, `where Self::Item: IntoIterator`, isn't given its items' iterator's drop by the impl's dictionary, so a caller through it drops each `Held` without its field's (#1080)",
    file: "src/lower/traits/drop_params.rs",
    find: "    for &own in tcx.associated_item_def_ids(id) {\n        if !matches!(tcx.def_kind(own), DefKind::AssocFn) {\n",
    replace: "    for &own in tcx.associated_item_def_ids(id) {\n        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^(blanket_)?impl_method_drops_its_items_iterator.rs$"],
  },
  {
    name: "from-iter-entry-given-no-trusted-len",
    breaks: "std's `from_iter` declares no `ITrustedLen` of its iterator, so a `Vec` entry can't make how std collects it and steps an endless `TrustedLen` before its \"capacity overflow\" (#1239)",
    file: "src/lower/traits/drop_params.rs",
    find: "                facts.extend(place.map(|place| (item_slot(tcx, method, place), TypeFact::TrustedLen)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_entry_of_an_adapter_generic_code_makes.rs$"],
  },
  {
    name: "generic-impl-trait-drop-unmade",
    breaks: "a dictionary leaves out the drop of a trait method's `impl Trait` of generics of its own, so what it holds, its `Loud`, is never dropped (#1575)",
    file: "src/lower/traits/drop_params.rs",
    find: "        for (item, item_ty) in self.item_types(tr).into_iter().chain(self.generic_impl_trait_types(tr)) {\n",
    replace: "        for (item, item_ty) in self.item_types(tr) {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_trait_of_a_method_with_its_own_generics_dropped.rs$"],
  },
];
