// Mutations of src/lower/traits/given.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-names-without-evidence",
    breaks:
      "a generic impl's dictionary a body asks twice, `holderTouch(TTouch)`, is written each time, as if the dictionary it's given weren't at its top (ADR 0979, amended, #691)",
    file: "src/lower/traits/given.rs",
    find: "        let given = (self.given.evidence.iter().map(|(_, value)| value))\n",
    replace: "        let given = (self.given.evidence.iter().take(0).map(|(_, value)| value))\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_dictionary_declared_once|copied_default_through_its_entry).rs$"],
    snapshots: true,
  },
  {
    name: "given-names-without-drops",
    breaks:
      "a generic impl's dictionary given a drop that generic code asks twice, `slotRenew(TDefault, dropT)`, is written each time, as if the drop it's given weren't at its top (ADR 0979, amended, #691)",
    file: "src/lower/traits/given.rs",
    find: "        let mut names: HashSet<String> = self.given_drops().into_iter().collect();\n",
    replace: "        let mut names: HashSet<String> = HashSet::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_dictionary_declared_once.rs$"],
    snapshots: true,
  },
  {
    name: "type-fact-of-wrong-kind",
    breaks: "`align_of::<T>()` reads the size its caller gave",
    file: "src/lower/traits/given.rs",
    find: "            .find(|&&(of, given, _)| matches!(of.kind(), ty::Param(p) if p.index == index) && given == fact)",
    replace:
      "            .find(|&&(of, _, _)| matches!(of.kind(), ty::Param(p) if p.index == index))",
    tests: ["test/corpus.test.ts", "-t", "type_facts"],
  },
  {
    name: "location-param-unnamed",
    breaks:
      "a function that takes its caller's location reads none it was given",
    file: "src/lower/traits/given.rs",
    find: "            self.given.caller_location = Some((id, Expr::var(&name)));\n",
    replace: "            let _ = &name;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "closure-reads-given-location",
    breaks:
      "`Location::caller()` in a closure in a tracked function is the function's caller's, not where it's written",
    file: "src/lower/traits/given.rs",
    find: "        (*owner == self.frame.owner).then(|| location.clone())\n",
    replace: "        let _ = owner;\n        Some(location.clone())\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "drop-slots-ungiven",
    breaks:
      "a function's associated types a caller could give a drop for aren't known as such, and one none gives is refused",
    file: "src/lower/traits/given.rs",
    find: "        self.give_drop_slots(drop_items(self.tcx, self.krate.foreign, id));\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(associated_into_iter|back_steps).rs$",
    ],
  },
  {
    name: "drop-evidence-unseen",
    breaks:
      "a dictionary given only its type parameter's drop is taken to take nothing, its function read as its object",
    file: "src/lower/traits/given.rs",
    find: "        self.takes_more_than_drops(id) || !self.drops_taken(id).is_empty()\n",
    replace: "        self.takes_more_than_drops(id)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_consts.rs$"],
  },
  {
    name: "bound-evidence-unseen",
    breaks:
      "a dictionary given its type parameter's dictionary is taken to take nothing, its function read as its object",
    file: "src/lower/traits/given.rs",
    find: "            || !bounds(self.tcx, self.krate.foreign, self.krate.given, id).is_empty()\n",
    replace: "",
    tests: [
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a dictionary that takes nothing",
    ],
  },
  {
    name: "const-param-evidence-unseen",
    breaks:
      "a dictionary given a const parameter's value is taken to take nothing",
    file: "src/lower/traits/given.rs",
    find: "        !const_params(self.tcx, id).is_empty()\n            || ",
    replace: "        ",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(array_vec_raw|associated_type_facts|borrow_mut_supertrait).rs$",
    ],
  },
  {
    name: "type-fact-evidence-unseen",
    breaks:
      "a dictionary given a type's fact, its size, is taken to take nothing",
    file: "src/lower/traits/given.rs",
    find: "            || !self.facts_asked(id).is_empty()\n",
    replace: "",
    tests: [
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a dictionary that takes nothing",
    ],
  },

  {
    name: "default-facts-kept",
    breaks:
      "a copied default's facts of its own type parameters stay given past it, so the next default's fact is refused as given twice",
    file: "src/lower/traits/given.rs",
    find: "        self.given.type_facts.truncate(scope.facts);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(generic_maps|item_keyed_map).rs$"],
  },
  {
    name: "default-consts-kept",
    breaks:
      "a copied default's const parameters stay given past it, read by what's lowered after it",
    file: "src/lower/traits/given.rs",
    find: "        self.given.const_params = scope.const_params;\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(const_generic_traits|copied_default_own_params).rs$",
    ],
    snapshots: true,
  },
  {
    name: "its-own-is-its-impls",
    breaks:
      "a fact a method asks of its own type parameter, a fold's `B`, is taken for its impl's, which a caller of a type it knows gives, so it's given one of `()` (#475)",
    file: "src/lower/traits/given.rs",
    find: "            || self.facts_asked(id).iter().any(|&(index, _)| index >= impls)\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "own fold asking of its own type parameter"],
  },
  {
    name: "default-own-params-numbered-as-the-impls",
    breaks:
      "a copied default's own type parameter, in the impl's terms, is numbered as it is in its trait's, so `U` of `counted_with<U>` is the impl's `U` of `Paired<I, U>` and drops with its drop (#804)",
    file: "src/lower/traits/given.rs",
    find: "        let index = (first + at - parent) as u32;\n",
    replace: "        let index = (at + first * 0) as u32;\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "default-own-bounds-not-in-its-env",
    breaks:
      "a copied default's type naming the trait method's own const parameter is asked about where only the impl's are, and rustc crashes",
    file: "src/lower/traits/given.rs",
    find: "        .chain(own.param_env.caller_bounds().iter())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^array_defaults.rs$"],
  },
  {
    name: "default-sized-not-impls",
    breaks:
      "a default's `Self: Sized` isn't the impl's `T: Sized`, and its trait on an adapter of `Self` is refused",
    file: "src/lower/traits/given.rs",
    find: "            (clause.as_trait_clause()).is_some_and(|tr| tcx.is_lang_item(tr.def_id(), LangItem::Sized))\n                || names_only(",
    replace: "            false\n                || names_only(",
    tests: ["test/corpus.test.ts", "-t", "^default_on_an_adapter_of_self.rs$"],
  },
  {
    name: "default-own-params-unbound-in-impl-terms",
    breaks:
      "a copied default's own parameter, numbered after the impl's, has none of its bounds there, so `Holder<T>` of its `T: Iterator<Item = Vec<u32>>` is cloned sharing its `Vec` (#804)",
    file: "src/lower/traits/given.rs",
    find: "                || names_only(clause, |index| index >= trait_count)\n",
    replace: "\n",
    tests: ["test/corpus.test.ts", "-t", "^default_method_clones.rs$"],
  },
  {
    name: "default-env-without-impls-bounds",
    breaks:
      "a copied default is lowered where only its own bounds hold, not its impl's, so a call of the impl's terms resolves to no impl: `Index<Id> for [T]` of its `[B]`, `Holder<I>`'s `Iterator`, `Map<T, F>: Counting`",
    file: "src/lower/traits/given.rs",
    find: "    let bounds = (ty::TypingEnv::post_analysis(tcx, imp).param_env.caller_bounds().iter())\n",
    replace: "    let bounds = (ty::List::<ty::Clause<'tcx>>::empty().iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_crate_index|copied_iterator_defaults|generic_maps|const_param_drops).rs$"],
  },
  {
    name: "default-impl-terms-twice",
    breaks:
      "what a copied default makes in its impl's terms, asked of the impl's drops, is put in those terms again, its own parameter numbered past the impl's read as one of the trait's, and rustc crashes (#804)",
    file: "src/lower/traits/given.rs",
    find: "        match self.given.self_args.filter(|_| !self.in_impl_drops()) {\n",
    replace: "        match self.given.self_args {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "default_own_param_numbered_as_the_impls"],
  },
];
