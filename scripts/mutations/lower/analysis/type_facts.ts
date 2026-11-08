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
    find: "                    ty::Ref(_, pointee, ty::Mutability::Mut) => pointee,\n",
    replace:
      "                    ty::Ref(_, _, ty::Mutability::Mut) => output,\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_made_by_entry.rs$"],
  },
  {
    name: "map-made-by-a-std-fn-unasked",
    breaks:
      "a map of a type parameter's keys a std function makes, `BTreeMap::new()`, `map(BTreeMap::from)`, has no class, its caller never asked, and is refused (ADR 0466)",
    file: "src/lower/analysis/type_facts.rs",
    find: "                if let Some((key, fact)) = map_of_param(tcx, made)\n",
    replace:
      "                if let Some((key, fact)) = map_of_param(tcx, made).filter(|_| false)\n",
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
    find: "                && (of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Align))\n",
    replace: "\n",
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
    name: "assoc-fact-unpassed",
    breaks:
      "a callee given `A::Item` that asks its size doesn't make its caller's dictionary carry it",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    changed |= carried.insert((assoc, fact));\n                }\n            }\n        }\n",
    replace:
      "                    let _ = (assoc, fact);\n                }\n            }\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "assoc-fact-impl-unasked",
    breaks:
      "an impl whose dictionary carries `T`'s size for `type Item = T` isn't given it",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if let ty::Param(param) = ty.kind() {\n                changed |= asked.insert((imp, param.index, fact));\n",
    replace:
      "            if let ty::Param(param) = ty.kind() {\n                let _ = (imp, param, fact);\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "unsized-name-unasked",
    breaks: "`type_name` of a `T: ?Sized` is asked of none, and refused",
    file: "src/lower/analysis/type_facts.rs",
    find: "                && (of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Align))\n",
    replace: "                && of.is_sized(tcx, typing_env)\n",
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
    find: "                && let Some(index) = slot_of(tcx, foreign, caller, key)\n",
    replace:
      "                && let Some(index) = slot_of(tcx, foreign, caller, key).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_maps.rs$"],
  },
  {
    name: "facts-not-passed-through-calls",
    breaks:
      "what a callee asks of a slot its caller's is given, `self.map(f).counts()`'s, isn't asked of the caller",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    changed |= ask((&mut asked, &mut built), caller, each);\n",
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
    name: "library-associated-uncarried",
    breaks: "a fact of a library trait's associated type, `size_of::<A::Item>()` of smallvec's code a consumer compiles, is no dictionary's, so it's refused, \"`size_of` of a type parameter\" (#476)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        ) if def_id.is_local() || foreign.is_library(def_id.krate) => Some(*def_id),\n",
    replace: "        ) if def_id.is_local() => Some(*def_id),\n",
    tests: ["test/crates.test.ts", "-t", "item_size_of_instance"],
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
    name: "uncarried-associated-read",
    breaks: "a consumer's fact of a library trait's associated type that the library's dictionaries don't carry is read of them, `undefined` (#476)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    assoc.is_local() || foreign.carried(assoc).contains(&fact)\n",
    replace: "    assoc.is_local() || foreign.is_library(assoc.krate) || foreign.carried(assoc).contains(&fact)\n",
    tests: ["test/crates.test.ts", "-t", "associated type its dictionaries don't carry"],
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
    name: "in-place-unasked",
    breaks:
      "generic code's `src.map(f).collect()` of items with a destructor asks its caller nothing of `I`, so it's refused, \"the buffer std collects an iterator in place in of a type parameter\" (#672)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    let source = held_facts(tcx, typing_env, iter, TypeFact::InPlace);\n",
    replace: "    let source = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-items-layout-unasked",
    breaks:
      "generic code's collect into a `Vec<T>` asks its caller no `TSize` or `TAlign`, so it's refused, \"`size_of` of a type parameter\" (#672)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            asked.extend(asked_of(tcx, foreign, typing_env, caller, (dest, fact), &gates));\n",
    replace: "            let _ = (dest, fact, &gates);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-default-unlifted",
    breaks:
      "a trait's default that collects `self` into a `Vec`, `sorted(self)`, compiled as a library, asks its blanket impl's dictionary nothing of `I`, so it's refused, \"the buffer std collects an iterator in place in of a type parameter\" (#672)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            asked.push((imp, each));\n",
    replace: "            let _ = each;\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_params.rs$"],
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
    name: "built-layout-ungated",
    breaks:
      "a built type's layout is read wherever its function is called, not only where its source's `IInPlace` is given, so a caller of a `LinkedList` collect giving `undefined` gives constants no collect reads (#737, #796)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        .filter(|gates| !gates.is_empty());\n",
    replace: "        .filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
    snapshots: true,
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
    name: "copied-default-analysed-in-its-own-env",
    breaks:
      "a trait's default copied into an impl collects in the impl's terms where only the impl's bounds hold, so what it collects isn't known there and asks nothing, and the collect is refused (#737)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        let impl_env = crate::lower::traits::default_typing_env(tcx, imp, caller, in_impl);\n",
    replace: "        let impl_env = ty::TypingEnv::post_analysis(tcx, imp);\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "copied-default-analysed-in-trait-numbers",
    breaks:
      "the analysis numbers a copied default's own parameters as its trait does, where its lowering numbers them after the impl's, so what it asks of them is another's (#804)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        let in_impl = crate::lower::traits::default_args(tcx, imp, caller, in_impl);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(in_place_collect_of_a_built_type_relayed|default_own_param_drops|default_own_param_numbered_as_the_impls).rs$"],
  },
  {
    name: "copied-default-own-asks-the-impls",
    breaks:
      "what a copied default asks of its own parameter, numbered after the impl's, is asked of the impl, which hasn't it, and rustc crashes (#804)",
    file: "src/lower/analysis/type_facts.rs",
    find: "            if !matches!(&each, Asked::Built(ty, ..) if names_params(*ty, |index| index >= first)) {\n",
    replace: "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "copied-default-own-asks-unasked",
    breaks:
      "what a copied default asks of its own parameter, the layout of a `Held<U>` it collects in place, is asked of no one, and the collect is refused (#804)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        if own {\n            asked.push((caller, each));\n",
    replace: "        if own && false {\n            asked.push((caller, each));\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "grown-size-unasked",
    breaks:
      "generic code's collect into a `Vec` of a type parameter's iterator asks its caller no `TSize`, so it's refused, \"`size_of` of a type parameter\" (#796)",
    file: "src/lower/analysis/type_facts.rs",
    find: "        asked.extend(grown_asked(recognition, typing_env, owner, (callee, args)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_grows_by_its_items_size.rs$"],
  },
  {
    name: "grown-size-of-a-built-type-unasked",
    breaks:
      "generic code's collect into a `Vec<(T, u8)>` of a type parameter's iterator asks its caller no layout of `(T, u8)`, so it's refused, \"`size_of` of a type parameter\" (#796)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    size.into_iter().collect()\n",
    replace: "    size.into_iter().filter(|each| matches!(each, Asked::Slot(..))).collect()\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_grows_by_its_items_size.rs$"],
  },
  {
    name: "built-slot-sort-unasked",
    breaks: "generic code's `sort_by_cached_key` of a key that holds a type parameter is refused, its `(K, usize)` pairs' sort asked of no caller (#925)",
    file: "src/lower/analysis/type_facts.rs",
    find: "    matches!(fact, TypeFact::Size | TypeFact::Align | TypeFact::Sort)",
    replace: "    matches!(fact, TypeFact::Size | TypeFact::Align)",
    tests: ["test/corpus.test.ts", "-t", "^sort_stable_seen.rs$"],
  },
];
