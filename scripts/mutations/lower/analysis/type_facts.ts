// Mutations of src/lower/analysis/type_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "struct-steps-own-params-unfilled",
    breaks:
      "a held struct's `fold`, whose own `B` and `F` its resolution needs, isn't followed: a `Zip`'s `fold` asks its sides' `TrustedLen` of no caller, refused (ADR 0975)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                        _ => tcx.mk_param_from_def(param),\n",
    replace:
      "                        _ => return tcx.lifetimes.re_erased.into(),\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "map-made-behind-mut-unasked",
    breaks:
      "a map of a type parameter's keys an entry's `or_default()` makes has no class, its caller never asked, and is refused (#314)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        ty::Ref(_, pointee, ty::Mutability::Mut) => pointee,\n",
    replace: "        ty::Ref(_, _, ty::Mutability::Mut) => output,\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_made_by_entry.rs$"],
  },
  {
    name: "map-made-by-a-std-fn-unasked",
    breaks:
      "a map of a type parameter's keys a std function makes, `BTreeMap::new()`, `map(BTreeMap::from)`, has no class, its caller never asked, and is refused (ADR 0466)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    (map_of_param(tcx, made).into_iter())\n",
    replace: "    (map_of_param(tcx, made).into_iter().filter(|_| false))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_map_made_by_entry|generic_maps).rs$",
    ],
  },
  {
    name: "type-facts-not-passed-on",
    breaks:
      "a library's impl the crate compiles itself, whose kept default asks `size_of` of the trait's `Self`, isn't given its own parameter's size (#1409)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                if asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {",
    replace:
      "                if false && asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {",
    tests: ["test/crates.test.ts", "-t", "library_instance_impl_asks_its_defaults_facts"],
  },
  {
    name: "type-facts-unsized-asked",
    breaks:
      "`size_of_val` of an unsized `T` asks its caller for a size its type hasn't",
    file: "src/lower/analysis/type_facts.rs",
    find: "                of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Room | TypeFact::Align)\n",
    replace: "                true\n",
    tests: ["test/diagnostics.test.ts", "-t", "size_of_val of an unsized"],
  },
  {
    name: "dyn-any-param-unasked",
    breaks: "a `T: 'static` made a `dyn Any` has no `TypeId` to give it",
    file: "src/lower/analysis/type_facts.rs",
    find: "                asking.asked.insert((caller, param.index, TypeFact::Id));\n                continue;",
    replace: "                continue;",
    tests: ["test/corpus.test.ts", "-t", "dyn_any_generic"],
  },
  {
    name: "any-bound-fact-asked",
    breaks: "a `T: Any` is given its `TypeId` beside its dictionary",
    file: "src/lower/analysis/type_facts.rs",
    find: "                && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_any.rs"],
    snapshots: true,
  },
  {
    name: "impl-method-facts-dropped",
    breaks:
      "an impl's method takes no fact of the impl's type parameter, so a call of it isn't given its `TSize`",
    file: "src/lower/analysis/type_facts.rs",
    find: "        if ask {\n            params.entry(id).or_default().push((index, fact));\n        }\n",
    replace:
      "        if ask && tcx.trait_impl_of_assoc(id).is_none() {\n            params.entry(id).or_default().push((index, fact));\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_type_facts.rs$"],
  },
  {
    name: "library-facts-unpassed",
    breaks:
      "a consumer's generic function that passes its `T` to a library's that asks its size doesn't ask it",
    file: "src/lower/analysis/type_facts.rs",
    find: "(caller, args), (slot, fact)) {\n                        self.ask(caller, each);\n",
    replace: "(caller, args), (slot, fact)) {\n                        let _ = each;\n",
    tests: ["test/crates.test.ts", "-t", "size of its consumer's type"],
  },
  {
    name: "bound-impl-facts-unasked",
    breaks:
      "a caller giving an impl's dictionary for a callee's bound, `C: Push<T>` of a `Stack<T>`, isn't asked what the impl asks of `T`",
    file: "src/lower/analysis/type_facts.rs",
    find: "        for bound in bounds(tcx, foreign, given, callee) {\n",
    replace:
      "            for bound in bounds(tcx, foreign, given, callee).into_iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_type_facts_through_trait.rs$"],
  },
  {
    name: "impl-facts-unlifted-in-fixpoint",
    breaks:
      "an impl's method's facts are the impl's only after the fixpoint, so a caller giving the impl's dictionary isn't asked them",
    file: "src/lower/analysis/type_facts.rs",
    find: "            changed |= asked.insert(fact);\n",
    replace: "            let _ = fact;\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_type_facts"],
  },
  {
    name: "assoc-fact-uncarried",
    breaks:
      "`size_of::<A::Item>()` isn't carried by `A: Array`'s dictionary, and is refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "                self.carried.insert((assoc, fact));\n",
    replace: "                let _ = (assoc, fact);\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "assoc-fact-impl-unasked",
    breaks:
      "an impl whose dictionary carries `T`'s size for `type Item = T` isn't given it",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if let Some(each) = asked_of(tcx, foreign, typing_env, imp, (ty, fact)) {\n",
    replace: "            if let Some(each) = None::<Asked<'tcx>>.filter(|_| typing_env == typing_env) {\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "unsized-name-unasked",
    breaks: "`type_name` of a `T: ?Sized` is asked of none, and refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "                of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Room | TypeFact::Align)\n",
    replace: "                of.is_sized(tcx, typing_env)\n",
    tests: ["test/corpus.test.ts", "-t", "^type_names_generic.rs$"],
  },
  {
    name: "held-param-name-unasked",
    breaks:
      "`type_name` of a type that holds a type parameter asks none of its name",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if fact == TypeFact::Name\n                && let Some(of) = args.types().last()\n",
    replace:
      "            if false\n                && let Some(of) = args.types().last()\n",
    tests: ["test/corpus.test.ts", "-t", "^type_names_generic.rs$"],
  },
  {
    name: "phantom-field-unnamed",
    breaks:
      "a derived `Debug` of a struct holding a `PhantomData<K>` is given no name of `K`, and refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "                for index in phantom_named(tcx, body.thir[source].ty) {\n",
    replace:
      "                for index in phantom_named(tcx, body.thir[source].ty).into_iter().take(0) {\n",
    tests: ["test/corpus.test.ts", "-t", "^phantom_debug.rs$"],
  },
  {
    name: "phantom-bound-unnamed",
    breaks:
      "`{:?}` of a `PhantomData<T>` written in place is given no name of `T`, and refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "            for index in debug_named(tcx, bound) {\n",
    replace:
      "            for index in debug_named(tcx, bound).into_iter().take(0) {\n",
    tests: ["test/corpus.test.ts", "-t", "^phantom_debug.rs$"],
  },
  {
    name: "map-class-not-asked",
    breaks:
      "a function that makes a map of its type parameter's keys isn't given their class",
    file: "src/lower/analysis/type_facts.rs",
    find: "        .filter_map(|(key, fact)| Some((slot_of(tcx, foreign, caller, key)?, fact)))\n",
    replace: "        .filter_map(|(key, fact)| Some((slot_of(tcx, foreign, caller, key).filter(|_| false)?, fact)))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_maps.rs$"],
  },
  {
    name: "facts-not-passed-through-calls",
    breaks:
      "what a callee asks of a slot its caller's is given, `self.map(f).counts()`'s, isn't asked of the caller",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    changed |= ask((&mut asked, &mut built, &mut carried), caller, each);\n",
    replace: "                    let _ = each;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_maps.rs$"],
  },
  {
    name: "trait-method-facts-unshared",
    breaks:
      "a trait method's own type parameter's fact isn't its impls' and its trait's",
    file: "src/lower/analysis/type_facts.rs",
    find: "            for id in implementors(tcx, method) {\n",
    replace:
      "            for id in implementors(tcx, method).into_iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_type_facts.rs$"],
  },
  {
    name: "trait-method-facts-dropped",
    breaks: "a trait method asks nothing of its own type parameters",
    file: "src/lower/analysis/type_facts.rs",
    find: "            (Some(_), _) => own || compiled,\n",
    replace: "            (Some(_), _) => compiled,\n",
    tests: ["test/corpus.test.ts", "-t", "^own_type_facts.rs$"],
  },
  {
    name: "sort-fact-unasked",
    breaks:
      "an unstable sort of a type parameter's items is given no `TSort`, and refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "            self.asked.insert((caller, index, TypeFact::Sort));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_unstable_sorts.rs$"],
  },
  {
    name: "fused-specialization-fact-not-asked",
    breaks:
      "a std body calling a std-private trait's method whose impl its parameter's being a `FusedIterator` picks, `FuseImpl::next`'s, doesn't ask `IFused` of it, so no caller gives it",
    file: "src/lower/analysis/type_facts.rs",
    find: "                if let Some(spec) = fact_specialization(tcx, typing_env, callee, args) {\n",
    replace:
      "                if let Some(spec) = fact_specialization(tcx, typing_env, callee, args).filter(|_| false) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "std-callee-not-followed",
    breaks:
      "a std body compiled with the crate is asked what its manifest says, which has none yet, not followed as the crate's own are: std's `fuse` doesn't ask what its `Fuse`'s `next` asks, `IFused`",
    file: "src/lower/analysis/type_facts.rs",
    find: "        if !callee.is_local() && !fns.contains_key(&callee) {\n",
    replace: "        if !callee.is_local() {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "std-provided-self-fact-not-asked",
    breaks:
      "a std trait's provided method compiled as a function over `Self` asks no fact of `Self`, so `fuse()` of a `FusedIterator` drops it as it ends",
    file: "src/lower/analysis/type_facts.rs",
    find: "            (Some(_), _) => own || compiled,\n",
    replace: "            (Some(_), _) => own,\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "impl-method-facts-not-asked",
    breaks:
      "a library impl method's facts, `Fuse<I>`'s `next_back`'s `IFused`, aren't asked of a caller of the trait's method, which gives `false`: a `FusedIterator`'s `Fuse` stepped from its back drops it as it ends",
    file: "src/lower/analysis/type_facts.rs",
    find: "                && instance.def_id() != callee\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^std_fuse.rs$"],
  },
  {
    name: "stepped-iterator-impl-facts-asked",
    breaks:
      "a caller asks the facts of `Fuse<I>`'s `next` impl, stepped as a JS iterator, so its impl methods take an `IFused` nothing reads",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if !crate::lower::recognition::stepped_as_js_iterator(tcx, callee, args)\n",
    replace: "            if true\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
    snapshots: true,
  },
  {
    name: "specialized-facts-unfollowed",
    breaks:
      "what both impls a fact picks between ask, `FlattenCompat`'s `size_hint`'s `ConstSize`, isn't asked of a `Flatten`'s, and std's compile refuses it",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if let Some(spec) = fact_specialization(tcx, typing_env, callee, args)\n                && fns.contains_key(&spec.selected.0)\n",
    replace:
      "            if let Some(spec) = fact_specialization(tcx, typing_env, callee, args).filter(|_| false)\n                && fns.contains_key(&spec.selected.0)\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "library-trait-impls-carry-none",
    breaks: "the crate's own impl of a library's trait carries none of what the library's code asks of its associated type, so `AArray.ItemSize` is `undefined` (#476)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            for fact in foreign.carried(item.def_id) {\n",
    replace: "            for fact in Vec::<TypeFact>::new() {\n",
    tests: ["test/crates.test.ts", "-t", "library_trait_facts_of_own_impl"],
  },
  {
    name: "trusted-len-of-a-collect-unasked",
    breaks: "a generic `collect()` into a `Vec` of a `take(n).rev()` of a type parameter isn't given whether the caller's iterator is `TrustedLen`, which its fold asks (#670)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                        asking.asked.insert((caller, index, TypeFact::TrustedLen));\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "impl-of-a-slot-type-unasked",
    breaks:
      "a caller giving an impl's dictionary for a type of its slots that isn't one of its type parameters, `impl<T: Iterator> Itertools for T` of `<I as IntoIterator>::IntoIter`, asks none of what the impl asks, and is refused, as itertools' `free::sorted` and semver's were (#794)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                true if given_params => self.calls.push((caller, impl_id, impl_args)),\n",
    replace: "                true if given_params => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "std-method-facts-unrelayed",
    breaks:
      "the analysis asks a std callee only its manifest's facts, none of std's, so `impl Extend for Either`'s `inner.extend(iter)` through `L: Extend<A>` asks its `T::IntoIter`'s in-place buffer of no caller, refused (#794)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    false => crate::lower::traits::foreign_facts(tcx, foreign, id),\n",
    replace: "                    false => foreign.type_facts(id),\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_extend.rs$"],
  },
  {
    name: "std-method-facts-of-resolved-calls",
    breaks:
      "a std trait's required method rustc resolves to std's own impl, `Vec::from_iter(it)`, relays its declaration's facts too, giving `sorted(it, IInPlace)` a parameter nothing reads (#794)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            let resolved = named.len() > 1;\n",
    replace: "            let resolved = false && named.len() > 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_called.rs$"],
    snapshots: true,
  },
  {
    name: "bound-collect-unrelayed",
    breaks:
      "a `collect()` into what the caller's own bound collects into, `Result<U, E>: FromIterator<..>`, asks nothing of the iterator, though the lowering calls its dictionary's `from_iter` with its in-place buffer: `try_collect` refused (#794)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                if bound.def_id == from_iterator && Some(callee) == collect_of(tcx) {\n",
    replace: "                if false && bound.def_id == from_iterator && Some(callee) == collect_of(tcx) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "tree-built-entries-unasked",
    breaks: "generic code's collect into a B-tree of a type parameter's keys asks its caller no `TStableSort`, and is refused (#926)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        if let Some(entries) = tree_built_entries(tcx, callee, args) {\n",
    replace: "        if let Some(entries) = tree_built_entries(tcx, callee, args).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_generic.rs$|^generic_btree_keys.rs$"],
  },
  {
    name: "super-item-uncarried",
    breaks: "a trait's default's `size_of` of `<Self as Iterator>::Item` isn't one its dictionaries carry, so the library's default compiled over `Self` is refused (#990)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        self.carried_super.insert((trait_id, assoc, fact));\n        // A library's trait's impls here carry what its manifest says.\n",
    replace: "        // A library's trait's impls here carry what its manifest says.\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "super-item-impls-unasked",
    breaks: "an impl that copies a default asking `size_of::<Self::Item>()` isn't asked its own type's, so the copy is refused (#990)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                self.asked.insert((imp, index, fact));\n",
    replace: "                let _ = index;\n",
    tests: ["test/corpus.test.ts", "-t", "^size_of_an_iterators_item.rs$"],
  },
  {
    name: "super-item-slot-unasked",
    breaks: "a generic function's `size_of::<I::Item>()` doesn't ask its slot, so it's refused (#990)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    None => _ = self.asked.insert((caller, index, fact)),\n",
    replace: "                    None => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^size_of_an_iterators_item.rs$"],
  },
  {
    name: "super-item-consumer-impls-carry-none",
    breaks: "a consumer's generic impl of a library's trait isn't asked its slot's facts its dictionary carries, so it's refused (#990)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            false => foreign.carried_super(trait_id),\n",
    replace: "            false => Vec::new(),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "map-made-unasked-of-mir",
    breaks: "a library's default compiled from its MIR doesn't ask the class of a map of `Self::Item` keys it makes, so a consumer's impl of the trait is refused, and a consumer's call of the library's copy gives none where it takes one (#990)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                // library's copies do (`map_made`, #990).\n                for (index, fact) in map_made(tcx, foreign, caller, (callee, args)) {\n",
    replace: "                // library's copies do (`map_made`, #990).\n                for (index, fact) in map_made(tcx, foreign, caller, (callee, args)).into_iter().filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_map_keys"],
  },
  {
    name: "eq-asks-of-mir-unread",
    breaks: "a library's generic `eq`, compiled by its consumer from its MIR, asks no `LengthFirst`, so the consumer's call is refused (#1182)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            for (callee, args) in named {\n                for index in eq_asks(recognition, typing_env, caller, (callee, args), &compares) {\n",
    replace: "            for (callee, args) in named {\n                for index in eq_asks(recognition, typing_env, caller, (callee, args), &compares).into_iter().filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "length_first_eq"],
  },
  {
    name: "default-call-facts-unasked-of-copies",
    breaks: "a trait's default that gives its `Self` to a function asking `IInPlace` of it is copied into a blanket impl that asks its caller nothing, so its call is refused, \"the buffer std collects an iterator in place in\", itertools' `multi_cartesian_product(self)` (#1313)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            let copies = copies[at].get_or_insert_with(|| copies_of_default(tcx, caller, args));\n",
    replace: "            let copies: Vec<collects::DefaultCopy<'tcx>> = Vec::new();\n            let _ = &copies[..];\n",
    tests: ["test/corpus.test.ts", "-t", "^default_gives_its_callee_what_self_collects_in_place.rs$"],
  },
  {
    name: "impl-carried-built-type-unasked",
    breaks: "an impl whose dictionary carries a fact of its associated type asks its makers for it only where that type is one of its own parameters, not built of them, itertools' `type Out = Result<U, E>`, so its dictionary is refused, \"`size_of` of a type parameter\" (#1348)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if let Some(each) = asked_of(tcx, foreign, typing_env, imp, (ty, fact)) {\n",
    replace: "            if let Some(each) = asked_of(tcx, foreign, typing_env, imp, (ty, fact)).filter(|each| matches!(each, Asked::Slot(..))) {\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_of_an_impls_own_associated_type.rs$"],
  },
  {
    name: "default-own-slot-asked-of-impl",
    breaks: "a copied default asks its impl a fact of a slot its own where clause names, `Self::Item: Hash`'s map class, so every caller of the impl's dictionary gives it, one whose items have no `Hash` refused where it calls only `ordered` (#1363)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                        .is_some_and(|index| index >= tcx.generics_of(caller).parent_count as u32)\n",
    replace: "                        .is_some_and(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^default_where_clause_facts_its_callers.rs$"],
  },

  {
    name: "instance-self-facts-unasked",
    breaks: "a library's default compiled here as a function over `Self` asks its callers nothing of `Self`, as a trait method's dictionary would give it, so its collect of `Self` is refused (#1370)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            crate::lower::recognition::is_compiled_std(tcx, id) || collects::instance_over_self(tcx, foreign, id);\n",
    replace: "            crate::lower::recognition::is_compiled_std(tcx, id);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_collects_self_in_place"],
  },
  {
    name: "mir-body-std-call-asks-unread",
    breaks: "a body read from its MIR alone, a library's default compiled here, asks nothing of a sort of its items, so itertools' `sorted` is refused, \"a stable sort of items that hold a type parameter\" (#1370)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                // What std's call asks of its items' type, a body read from\n                // MIR alone too, a library's default compiled here (#1370).\n                if asking.std_call_asks((tcx, foreign), typing_env, caller, (callee, args)) {\n                    continue;\n                }\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_collects_self_in_place"],
  },
  {
    name: "library-impl-associated-slot-unrelayed",
    breaks: "a library impl's fact of a std associated type of its parameter, `<T as Iterator>::Item` of `impl<T: Iterator> Tools for T`, isn't asked of a generic caller giving `<I as Iterator>::Item`, so its dictionary is refused, \"`size_of` of an associated type whose facts its caller's dictionary doesn't carry\" (#1401)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            let relayed = |at: usize| of_slots(at) || at >= impl_args.len();\n",
    replace: "            let relayed = |at: usize| of_slots(at);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_given_an_associated_types_size$"],
  },
  {
    name: "instance-impl-defaults-unasked",
    breaks:
      "a library's impl the crate compiles itself calls the defaults it keeps, compiled over `Self`, but asks its callers none of their facts of `Self`, so the dictionary's entry reads one it isn't given: itertools' test_core refused, \"`whether it's a `TrustedLen`` of a type parameter\" (#1409)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    passed.push(((imp, param.index), (item.def_id, index as u32)));\n",
    replace: "                    let _ = (imp, param, index, &passed);\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_instance_impl_asks_its_defaults_facts$"],
  },
  {
    name: "instance-impl-super-items-unasked",
    breaks:
      "a library's impl the crate compiles itself isn't asked what its dictionary carries of `<T as Iterator>::Item`, so a kept default's `size_of` of it is refused, \"an associated type whose facts its caller's dictionary doesn't carry\" (#1409)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    for &(trait_id, imp) in &lowered {\n        let facts = match trait_id.is_local() {\n",
    replace: "    for &(trait_id, imp) in lowered.iter().filter(|(_, imp)| imp.is_local()) {\n        let facts = match trait_id.is_local() {\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_instance_impl_asks_its_defaults_facts$"],
  },
  {
    name: "rc-instance-unasked",
    breaks: "generic code's `Rc::new` of a type parameter a counted class may be asks its caller nothing, so it's given no `TInstance` and is refused (#1423)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    .then_some(pointee)\n",
    replace: "    .then_some(pointee)\n    .filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^class_rc_made_by_generic_code.rs$"],
  },
  {
    name: "rc-new-cyclic-instance-unasked",
    breaks: "generic code's `Rc::new_cyclic` of a type parameter a counted class may be asks its caller nothing, so it's given no `TInstance` and is refused (#1423)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        _ if new_cyclic => args.type_at(0),\n",
    replace: "        _ if false && new_cyclic => args.type_at(0),\n",
    tests: ["test/corpus.test.ts", "-t", "^class_rc_made_by_generic_code.rs$"],
  },
  {
    name: "rc-instance-asked-by-classify",
    breaks: "every call is classified to find `Rc::new_cyclic`, a library's const generic's too, whose arguments `classify` resolves in no environment of their own: rustc panics, `cannot find` the const parameter (#1423)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        _ if new_cyclic => args.type_at(0),\n",
    replace: "        _ if new_cyclic || recognition.classify(callee, args).is_some() && false => args.type_at(0),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: const_generic_(with_type|inside)$"],
  },
  {
    name: "room-and-size-both-taken",
    breaks: "a slot asked both its `Size` and its `Room`, named alike, is taken twice, `TSize` twice, which isn't JS (#1260)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        settle_room(facts);\n",
    replace: "",
    tests: ["test/vec-capacity.test.ts", "-t", "one TSize"],
  },
  {
    name: "impl-entries-ask-nothing",
    breaks: "an impl's supertrait and item-bound dictionaries ask their makers nothing, so the size of an associated type a supertrait's impl gives, `BitOps0: AndNot<Output = Self>` of an `X2<W, G>`, is refused (#1430)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        asking.of_impl_entries(tcx, foreign, trait_id, imp);\n",
    replace: "        let _ = (trait_id, imp);\n",
    tests: ["test/corpus.test.ts", "-t", "^supertrait_dictionary_asks_its_makers.rs$"],
  },
  {
    name: "dictionary-of-callers-params-relays-no-layout",
    breaks: "generic code that makes an impl's dictionary of its own type parameters, `X2<W, u8>`, relays none of the layouts of the types the impl builds of them, and is refused (#1430)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                true if given_params => self.calls.push((caller, impl_id, impl_args)),\n",
    replace: "                true if given_params && impl_args.iter().any(|arg| arg.as_type().is_some_and(|ty| !matches!(ty.kind(), ty::Param(_)) && ty.has_param())) => self.calls.push((caller, impl_id, impl_args)),\n",
    tests: ["test/corpus.test.ts", "-t", "^supertrait_dictionary_asks_its_makers.rs$"],
  },
  {
    name: "impl-method-built-layout-unlifted",
    breaks: "an impl's method's layout of a type built of the impl's parameters, quickcheck's `(K, V)` of its `BTreeMap<K, V>`, is asked of no maker, and the impl is refused (#1420)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        for (impl_id, ty, facts) in lifted_built {\n",
    replace: "        for (impl_id, ty, facts) in lifted_built.into_iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_asks_the_layout_of_a_type_of_its_parameters.rs$"],
  },
  {
    name: "lend-offset-never-asked",
    breaks: "generic code's lend from inside a receiver whose instances keep it at different offsets asks no offset of its callers, so it's refused (#1478)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            for (receiver, lend) in lend_offsets_asked(tcx, (recognition, stored), caller, (callee, args)) {\n",
    replace: "            for (receiver, lend) in lend_offsets_asked(tcx, (recognition, stored), caller, (callee, args)).into_iter().take(0) {\n",
    tests: ["test/corpus.test.ts","-t","^generic_lend_offsets_given_by_callers.rs$"],
  },
  {
    name: "lend-offset-unseen-callers-unasked",
    breaks: "a library's generic lend, whose callers it doesn't see, asks no offset of its consumers, so it's refused (#1478)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            let Some(instances) = &instances else { return true };\n",
    replace: "            let Some(instances) = &instances else { return false };\n",
    tests: ["test/crates.test.ts","-t","two crates: lend_offset_a_consumer_gives"],
  },
];
