// Mutations of crates/rust-js-lower/src/traits/given.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-names-without-evidence",
    breaks:
      "a generic impl's dictionary a body asks twice, `holderTouch(TTouch)`, is written each time, as if the dictionary it's given weren't at its top (ADR 0979, amended, #691)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        let given = (self.given.evidence.iter().map(|(_, value)| value))\n",
    replace: "        let given = (self.given.evidence.iter().take(0).map(|(_, value)| value))\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_dictionary_declared_once|copied_default_through_its_entry).rs$"],
    snapshots: true,
  },
  {
    name: "given-names-without-drops",
    breaks:
      "a generic impl's dictionary given a drop that generic code asks twice, `slotRenew(TDefault, dropT)`, is written each time, as if the drop it's given weren't at its top (ADR 0979, amended, #691)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        let mut names: IndexSet<js::Name> = self.given_drops().into_iter().collect();\n",
    replace: "        let mut names: IndexSet<js::Name> = IndexSet::default();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_dictionary_declared_once.rs$"],
    snapshots: true,
  },
  {
    name: "type-fact-of-wrong-kind",
    breaks: "`align_of::<T>()` reads the size its caller gave",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            .filter(|&&(_, given, _)| given == fact)\n            .find(|&&(at, _, _)| normal(at) == of)\n",
    replace: "            .find(|&&(at, _, _)| normal(at) == of)\n",
    tests: ["test/corpus.test.ts", "-t", "type_facts"],
  },
  {
    name: "location-param-unnamed",
    breaks:
      "a function that takes its caller's location reads none it was given",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            self.given.caller_location = Some((id, Expr::var(&name)));\n",
    replace: "            let _ = &name;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "closure-reads-given-location",
    breaks:
      "`Location::caller()` in a closure in a tracked function is the function's caller's, not where it's written",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        (*owner == self.frame.owner).then(|| location.clone())\n",
    replace: "        let _ = owner;\n        Some(location.clone())\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "drop-slots-ungiven",
    breaks:
      "a function's associated types a caller could give a drop for aren't known as such, and one none gives is refused",
    file: "crates/rust-js-lower/src/traits/given.rs",
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
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        self.takes_more_than_drops(id) || !self.drops_taken(id).is_empty()\n",
    replace: "        self.takes_more_than_drops(id)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_consts.rs$"],
  },
  {
    name: "bound-evidence-unseen",
    breaks:
      "a dictionary given its type parameter's dictionary is taken to take nothing, its function read as its object",
    file: "crates/rust-js-lower/src/traits/given.rs",
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
    file: "crates/rust-js-lower/src/traits/given.rs",
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
    file: "crates/rust-js-lower/src/traits/given.rs",
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
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        self.given.type_facts.truncate(scope.facts);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(generic_maps|item_keyed_map).rs$"],
  },
  {
    name: "default-consts-kept",
    breaks:
      "a copied default's const parameters stay given past it, read by what's lowered after it",
    file: "crates/rust-js-lower/src/traits/given.rs",
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
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            || (self.facts_asked(id).iter()).any(|&(index, _)| of_its_own(drop_slot(self.tcx, self.slots(), id, index)))\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "own fold asking of its own type parameter"],
  },
  {
    name: "default-own-params-numbered-as-the-impls",
    breaks:
      "a copied default's own type parameter, in the impl's terms, is numbered as it is in its trait's, so `U` of `counted_with<U>` is the impl's `U` of `Paired<I, U>` and drops with its drop (#804)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        let index = (first + at - parent) as u32;\n",
    replace: "        let index = (at + first * 0) as u32;\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "default-env-without-impls-bounds",
    breaks:
      "a copied default is lowered where only its own bounds hold, not its impl's, so a call of the impl's terms resolves to no impl: `Index<Id> for [T]` of its `[B]`, `Holder<I>`'s `Iterator`, `Map<T, F>: Counting`",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "    let bounds = (impl_env.param_env.caller_bounds())\n",
    replace: "    let bounds = (ty::List::<ty::Clause<'tcx>>::empty().iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_crate_index|copied_iterator_defaults|generic_maps|const_param_drops).rs$"],
  },
  {
    name: "default-env-without-its-own-clauses",
    breaks:
      "a copied default is lowered where its own where clause doesn't hold, `T::Item: IntoIterator` of its `Self::Item: IntoIterator`, its own parameters' bounds, its `Self: Sized`, so the `Map<T, F>` it counts with resolves to no impl, and its own `T`'s `Holder<T>` clones sharing its `Vec` (#803, #804)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        .chain(in_impl)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(generic_maps|default_method_clones|default_on_an_adapter_of_self|array_defaults).rs$"],
  },
  {
    name: "default-env-clauses-unnormalized",
    breaks:
      "a copied default's where clause of a non-blanket impl isn't in the impl's terms as its body is, `<Wrapper<I> as Iterator>::Item: IntoIterator`, so its `Held`'s drop resolves to nothing (#803)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        (tcx.try_normalize_erasing_regions(impl_env, ty::Unnormalized::new_wip(clause))).unwrap_or(clause)\n",
    replace: "        clause\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_of_a_non_blanket_impl_steps_its_self).rs$"],
  },
  {
    name: "default-mir-unnormalized",
    breaks:
      "a copied default's MIR in its impl's terms isn't normalized there, so `<Enumerated<P> as Pull>::Out` is refused (#803)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        ty_op: |t: Ty<'tcx>| match t.has_aliases() && !t.has_escaping_bound_vars() {\n",
    replace: "        ty_op: |t: Ty<'tcx>| match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(blanket_iterator_trait|generic_associated_types).rs$"],
  },
  {
    name: "given-fact-unnormalized",
    breaks:
      "a fact given is found only by its type as written, so a copied default whose where clause says `Self::Item` is `T::Item` is refused `size_of` of what it was given (#828), and a copied default's `<W<I> as Iterator>::Item` of `impl<I, T> Tools for W<I>` isn't the impl's `T`'s (#2012)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            .find(|&&(at, _, _)| normal(at) == of)\n",
    replace: "            .find(|&&(at, _, _)| self.tcx.erase_and_anonymize_regions(at) == of)\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_matches_its_tuple_whole|default_gives_a_callee_its_selfs_items_of_an_impls_own_parameter).rs$"],
  },
  {
    name: "copied-closure-own-generics-dropped",
    breaks:
      "a closure made in a copied default is read with the default's arguments alone, and rust-js crashes where its MIR names its own generics, `minmax_by`'s `|_| ()` (#828)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "    tcx.mk_args_from_iter(args.iter().chain(own.iter().skip(args.len())))\n",
    replace: "    tcx.mk_args_from_iter(args.iter().chain(own.iter().skip(own.len())))\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_default_closure_of_its_own_generics.rs$"],
  },
  {
    name: "super-item-unread",
    breaks: "a library's default compiled over `Self` doesn't read `<Self as Iterator>::Item`'s facts of its dictionary, so it's refused (#990)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            || !self.carried_super(carrier).contains(&(def_id, fact))\n",
    replace: "            || true\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "own-slot-fact-of-the-impl-counted-own",
    breaks: "an iterator's own `last` that asks a fact of its impl's `I::Item`, a collect's layout, is taken for one asking a fact of its own type parameters, so `Wrap<I>`'s `last` through a std adapter, itertools' `MultiProduct` collected, is refused (#840)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            || (self.facts_asked(id).iter()).any(|&(index, _)| of_its_own(drop_slot(self.tcx, self.slots(), id, index)))\n",
    replace: "            || self.facts_asked(id).iter().any(|&(index, _)| index >= impls)\n",
    tests: ["test/corpus.test.ts", "-t", "^own_last_of_its_impls_item_layout.rs$"],
  },
  {
    name: "declared-param-in-impls-terms",
    breaks: "a trait's default copied into an impl reads its own type parameters' facts at the impl's numbering, so `T`'s `Some(None)` is unboxed beside a `U` never `None` (#2273)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "            Some(own) => Declared::Own(owner, own),\n",
    replace: "            Some(_) => Declared::Own(owner, index),\n",
    tests: ["test/corpus.test.ts", "-t", "^default_copied_reads_its_own_params.rs$"],
  },
  {
    name: "declared-ty-in-impls-terms",
    breaks: "a copied default's own `I: DoubleEndedIterator` is asked of the default's clauses at the impl's numbering, which names none, and its `rev()` is refused (#2273)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "                    Declared::Own(_, index) => Ty::new_param(tcx, index, param.name),\n",
    replace: "                    Declared::Own(..) => part,\n",
    tests: ["test/corpus.test.ts", "-t", "^default_copied_reads_its_own_params.rs$"],
  },
  {
    name: "trait-method-param-unshifted",
    breaks: "an impl's method's own type parameter is lifted to its trait's method at the impl's numbering, past no `Self`, so a generic call's requirement falls on `U` where it's `I` (#2276)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "    index.checked_sub(first).map(|own| (item, parent + own))\n",
    replace: "    index.checked_sub(first).map(|own| (item, own))\n",
    tests: ["test/diagnostics.test.ts", "-t", "a trait's method that takes no destructor"],
  },
  {
    name: "given-location-unset",
    breaks: "a step table's entry given the location by the runtime calls its impl with the place the table is made (#2324)",
    file: "crates/rust-js-lower/src/traits/given.rs",
    find: "        let outer = self.given.caller_location.replace((self.frame.owner, location));\n",
    replace: "        let outer = self.given.caller_location.clone();\n        let _ = location;\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
];
