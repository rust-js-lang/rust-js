// Mutations of src/lower/analysis/type_facts/collects.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "in-place-unasked",
    breaks:
      "generic code's `src.map(f).collect()` of items with a destructor asks its caller nothing of `I`, so it's refused, \"the buffer std collects an iterator in place in of a type parameter\" (#672)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "    let source = held_facts(tcx, typing_env, iter, TypeFact::InPlace);\n",
    replace: "    let source = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-items-layout-unasked",
    breaks:
      "generic code's collect into a `Vec<T>` asks its caller no `TSize` or `TAlign`, so it's refused, \"`size_of` of a type parameter\" (#672)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "            asked.extend(asked_of(tcx, foreign, typing_env, caller, (dest, fact), &gates));\n",
    replace: "            let _ = (dest, fact, &gates);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-default-unlifted",
    breaks:
      "a trait's default that collects `self` into a `Vec`, `sorted(self)`, compiled as a library, asks its blanket impl's dictionary nothing of `I`, so it's refused, \"the buffer std collects an iterator in place in of a type parameter\" (#672)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "            asked.push((imp, each));\n",
    replace: "            let _ = each;\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_params.rs$"],
  },
  {
    name: "built-layout-ungated",
    breaks:
      "a built type's layout is read wherever its function is called, not only where its source's `IInPlace` is given, so a caller of a `LinkedList` collect giving `undefined` gives constants no collect reads (#737, #796)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "        .filter(|gates| !gates.is_empty());\n",
    replace: "        .filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
    snapshots: true,
  },
  {
    name: "copied-default-analysed-in-its-own-env",
    breaks:
      "a trait's default copied into an impl collects in the impl's terms where only the impl's bounds hold, so what it collects isn't known there and asks nothing, and the collect is refused (#737)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "            typing_env: crate::lower::traits::default_typing_env(tcx, imp, default, in_impl),\n",
    replace: "            typing_env: ty::TypingEnv::post_analysis(tcx, imp),\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "copied-default-analysed-in-trait-numbers",
    breaks:
      "the analysis numbers a copied default's own parameters as its trait does, where its lowering numbers them after the impl's, so what it asks of them is another's (#804)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "        let in_impl = crate::lower::traits::default_args(tcx, imp, default, in_impl);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(in_place_collect_of_a_built_type_relayed|default_own_param_drops|default_own_param_numbered_as_the_impls).rs$"],
  },
  {
    name: "copied-default-own-asks-the-impls",
    breaks:
      "what a copied default asks of its own parameter, numbered after the impl's, is asked of the impl, which hasn't it, and rustc crashes (#804)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "            if !matches!(&each, Asked::Built(ty, ..) if names_params(*ty, |index| index >= first)) {\n",
    replace: "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "copied-default-own-asks-unasked",
    breaks:
      "what a copied default asks of its own parameter, the layout of a `Held<U>` it collects in place, is asked of no one, and the collect is refused (#804)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "        if own {\n            asked.push((caller, each));\n",
    replace: "        if own && false {\n            asked.push((caller, each));\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "grown-size-unasked",
    breaks:
      "generic code's collect into a `Vec` of a type parameter's iterator asks its caller no `TSize`, so it's refused, \"`size_of` of a type parameter\" (#796)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "        asked.extend(grown_asked(\n            recognition,\n            typing_env,\n            owner,\n            (callee, args),\n            keeps_capacity,\n        ));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_grows_by_its_items_size.rs$"],
  },
  {
    name: "grown-size-of-a-built-type-unasked",
    breaks:
      "generic code's collect into a `Vec<(T, u8)>` of a type parameter's iterator asks its caller no layout of `(T, u8)`, so it's refused, \"`size_of` of a type parameter\" (#796)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "    size.into_iter().collect()\n",
    replace: "    size.into_iter().filter(|each| matches!(each, Asked::Slot(..))).collect()\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_grows_by_its_items_size.rs$"],
  },
  {
    name: "super-item-collect-uncarried",
    breaks: "a trait's default's collect of `Self` isn't given its items' layout by its dictionary, so the library's default compiled over `Self` is refused (#990)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "                    carried_super.insert((trait_id, assoc, *fact));\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "vec-growth-size-unasked",
    breaks: "in a crate that keeps capacities, generic code that pushes to a `Vec<T>` asks its caller no `TSize`, so it's refused, \"`size_of` of a type parameter\" (#1180)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "    if let Some(item) = grown {\n",
    replace: "    if let Some(item) = grown.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity_grown.rs$"],
  },
  {
    name: "vec-collect-unasked",
    breaks: "generic code's `collect()` into a `Vec` asks its caller no `IVecCollect`, so it collects as it did, an endless `TrustedLen` stepped before its \"capacity overflow\" (#1239)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "            Some(this) => Vec::from_iter(asked_of(\n",
    replace: "            Some(_) if true => Vec::new(),\n            Some(this) => Vec::from_iter(asked_of(\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collect_as_its_caller_knows.rs$"],
  },
  {
    name: "vec-collect-asks-in-place-too",
    breaks: "a collect `IVecCollect` answers asks `IInPlace` of its iterator too, a parameter its JS takes for nothing (#1239)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "        let mut asked = match vec_collected(recognition, typing_env, (callee, args)) {\n",
    replace: "        let mut asked = match vec_collected(recognition, typing_env, (callee, args)).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collect_as_its_caller_knows.rs$"],
  },
  {
    name: "instance-default-asked-through-copies",
    breaks: "a library's default the crate compiles itself as a function over `Self`, an instance, is asked its collect's facts only through the crate's impls that copy it, none, so its `collect()` of `Self` into a `Vec` is refused, \"the buffer std collects an iterator in place in\", itertools' `collect_vec` (#1370)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "        .filter(|_| tcx.defaultness(caller).has_value() && !instance_over_self(tcx, foreign, caller));\n",
    replace: "        .filter(|_| tcx.defaultness(caller).has_value());\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_collects_self_in_place"],
  },  {
    name: "instance-over-self-though-copied",
    breaks: "a library's default the crate compiles itself is a function over `Self` asking its own facts though the crate's impls copy it, whose dictionaries give them already, so each impl's entry is refused, \"a trait method's type parameter's fact here\" (#990, #1370)",
    file: "src/lower/analysis/type_facts/collects.rs",
    find: "            .all(|&imp| tcx.impl_item_implementor_ids(imp.to_def_id()).contains_key(&default))\n",
    replace: "            .all(|_| true)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts$"],
  },
];
