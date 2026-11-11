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
    find: "    let (key, fact) = map_of_param(tcx, made)?;\n",
    replace: "    let (key, fact) = map_of_param(tcx, made).filter(|_| false)?;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_map_made_by_entry|generic_maps).rs$",
    ],
  },
  {
    name: "type-facts-not-passed-on",
    breaks:
      "a generic function passing its `U` on to one that asks `size_of::<T>()` isn't given `U`'s size",
    file: "src/lower/analysis/type_facts.rs",
    find: "                if asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {",
    replace:
      "                if false && asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {",
    tests: ["test/corpus.test.ts", "-t", "type_facts"],
  },
  {
    name: "type-facts-unsized-asked",
    breaks:
      "`size_of_val` of an unsized `T` asks its caller for a size its type hasn't",
    file: "src/lower/analysis/type_facts.rs",
    find: "                |of: ty::Ty<'tcx>| of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Align);\n",
    replace: "                |_: ty::Ty<'tcx>| true;\n",
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
    find: "(slot, fact, &gates)) {\n                        self.ask(caller, each);\n",
    replace: "(slot, fact, &gates)) {\n                        let _ = each;\n",
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
    find: "            if let Some(each) = asked_of(tcx, foreign, typing_env, imp, (ty, fact), &None) {\n",
    replace: "            if let Some(each) = None::<Asked<'tcx>>.filter(|_| typing_env == typing_env) {\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "unsized-name-unasked",
    breaks: "`type_name` of a `T: ?Sized` is asked of none, and refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "                |of: ty::Ty<'tcx>| of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Align);\n",
    replace: "                |of: ty::Ty<'tcx>| of.is_sized(tcx, typing_env);\n",
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
    find: "    Some((slot_of(tcx, foreign, caller, key)?, fact))\n",
    replace: "    Some((slot_of(tcx, foreign, caller, key).filter(|_| false)?, fact))\n",
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
    find: "                asking.asked.insert((caller, index, TypeFact::Sort));\n",
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
    name: "library-fact-of-alias-not-asked",
    breaks:
      "a library function's fact of a std associated type of a caller's parameter, `I::IntoIter`'s `IFused`, isn't asked",
    file: "src/lower/analysis/type_facts.rs",
    find: "    if let Some(slot) = slot_of(tcx, foreign, caller, of) {\n",
    replace: "    if let Some(slot) = slot_of(tcx, foreign, caller, of).filter(|_| matches!(of.kind(), ty::Param(_))) {\n",
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
    name: "built-layout-unasked",
    breaks:
      "a collect's items of a type built of its function's type parameters, `Held<<H::Item as IntoIterator>::IntoIter>`, ask no caller their layout, and the collect is refused, `size_of` of a type parameter, as itertools' `multi_cartesian_product` was (#737)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        && tcx.layout_of(typing_env.as_query_input(ty)).is_err()\n",
    replace: "        && tcx.layout_of(typing_env.as_query_input(ty)).is_err()\n        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type.rs$"],
  },
  {
    name: "built-layout-asked-though-never-read",
    breaks:
      "a generic caller giving a built slot's every gate a source std never collects in place still asks the layout of its own caller, which no collect reads (#737)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                (false, true) => return Vec::new(),\n",
    replace: "                (false, true) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
    snapshots: true,
  },
  {
    name: "built-layout-unrelayed",
    breaks:
      "a generic caller of a function asking a built type's layout, of a type built of its own slots, `Held<IntoIter<T>>`, asks its own caller none, and its call is refused (#737)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        .filter_map(|each| asked_of(tcx, foreign, typing_env, caller, each, &gates))\n",
    replace:
      "        .filter_map(|(of, fact)| Some(Asked::Slot(slot_of(tcx, foreign, caller, of)?, fact)).filter(|_| !gates.is_some()))\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "impl-of-a-slot-type-unasked",
    breaks:
      "a caller giving an impl's dictionary for a type of its slots that isn't one of its type parameters, `impl<T: Iterator> Itertools for T` of `<I as IntoIterator>::IntoIter`, asks none of what the impl asks, and is refused, as itertools' `free::sorted` and semver's were (#794)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                true => self.calls.push((caller, impl_id, impl_args)),\n",
    replace: "                true => {}\n",
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
    name: "built-slot-sort-unasked",
    breaks: "generic code's `sort_by_cached_key` of a key that holds a type parameter is refused, its `(K, usize)` pairs' sort asked of no caller (#925)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        TypeFact::Size | TypeFact::Align | TypeFact::Sort | TypeFact::StableSort\n",
    replace: "        TypeFact::Size | TypeFact::Align | TypeFact::StableSort\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_stable_seen.rs$"],
  },
  {
    name: "built-slot-stable-sort-unasked",
    breaks: "generic code's collect into a `BTreeMap<K, V>` of a type parameter's keys is refused, its `(K, V)` entries' stable sort asked of no caller (#926)",
    file: "src/lower/analysis/type_facts.rs",
    find: " | TypeFact::Sort | TypeFact::StableSort\n",
    replace: " | TypeFact::Sort\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_generic.rs$|^generic_collect_grows_by_its_items_size.rs$"],
  },
  {
    name: "tree-built-entries-unasked",
    breaks: "generic code's collect into a B-tree of a type parameter's keys asks its caller no `TStableSort`, and is refused (#926)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if let Some(entries) = tree_built_entries(tcx, callee, args) {\n",
    replace: "            if let Some(entries) = tree_built_entries(tcx, callee, args).filter(|_| false) {\n",
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
    find: "                if let Some((index, fact)) = map_made(tcx, foreign, caller, (callee, args)) {\n",
    replace: "                if let Some((index, fact)) = map_made(tcx, foreign, caller, (callee, args)).filter(|_| false) {\n",
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
    name: "callee-fact-of-assoc-uncarried",
    breaks: "a fact a std callee asks of an associated type no slot names, a flatten's items' constant size of `<S as Source>::Items`, isn't carried by its trait's dictionaries, so it's refused, \"which no caller gives\" (#1312, #1341)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        return Some(Asked::Carried(assoc, fact));\n",
    replace: "        return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_a_traits_items_sized_by_its_dictionary.rs$"],
  },
  {
    name: "default-call-facts-unasked-of-copies",
    breaks: "a trait's default that gives its `Self` to a function asking `IInPlace` of it is copied into a blanket impl that asks its caller nothing, so its call is refused, \"the buffer std collects an iterator in place in\", itertools' `multi_cartesian_product(self)` (#1313)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            let copies = copies_of_default(tcx, caller, args);\n",
    replace: "            let copies: Vec<collects::DefaultCopy<'tcx>> = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^default_gives_its_callee_what_self_collects_in_place.rs$"],
  },
  {
    name: "impl-carried-built-type-unasked",
    breaks: "an impl whose dictionary carries a fact of its associated type asks its makers for it only where that type is one of its own parameters, not built of them, itertools' `type Out = Result<U, E>`, so its dictionary is refused, \"`size_of` of a type parameter\" (#1348)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if let Some(each) = asked_of(tcx, foreign, typing_env, imp, (ty, fact), &None) {\n",
    replace: "            if let Some(each) = asked_of(tcx, foreign, typing_env, imp, (ty, fact), &None).filter(|each| matches!(each, Asked::Slot(..))) {\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_of_an_impls_own_associated_type.rs$"],
  },
];
