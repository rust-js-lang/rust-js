// Mutations of src/lower/analysis/iterator_steps.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const adapters = ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"];
const extensions = [
  "test/corpus.test.ts",
  "-t",
  "^given_iterator_extensions.rs$",
];

export const mutations: Mutation[] = [
  {
    name: "item-param-stepped-unseen",
    breaks:
      "an impl whose associated type is its own type parameter, `type Iter = I`, isn't given `I`'s dictionaries, which the JS iterator its dictionary makes of it steps by: refused (#972)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                stepped.extend(extensions.into_iter().map(|tr| (imp, param.index, tr)));\n",
    replace: "                    let _ = extensions;\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_given_as_it_is.rs$"],
  },

  {
    name: "std-body-callee-steps-unpassed",
    breaks:
      "std's `Skip::rfold`, compiled, calls `self.try_rfold(..)`, which steps `I` from its back, but isn't given `I`'s `DoubleEndedIterator`: std's bodies don't compile (ADR 0927)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                                if fns.contains_key(&instance.def_id()) {\n                                    pass_on(&mut passed, instance.def_id(), instance.args);\n                                } else if foreign.item(instance.def_id()).is_some() {\n",
    replace:
      "                                if false {\n                                    pass_on(&mut passed, instance.def_id(), instance.args);\n                                } else if foreign.item(instance.def_id()).is_some() {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "stepped-param-unseen",
    breaks:
      "an adapter's `next`, which steps its `I`, is given no dictionary for it",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    stepped.extend(by.iter().map(|&by| (caller, param.index, by)));\n",
    replace: "",
    tests: adapters,
  },
  {
    name: "stepped-through-impl-unseen",
    breaks:
      "`skip_one`, which steps a `Doubled<I>` by its impl's `next`, is given no dictionary for `I`",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    pass_on(&mut passed, instance.def_id(), instance.args);\n                }\n                continue;\n",
    replace: "                }\n                continue;\n",
    tests: adapters,
  },
  {
    name: "stepped-passed-on-unseen",
    breaks:
      "a function that passes its `I` to one that steps it is given no dictionary for it",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                if stepped.insert((from.0, from.1, by)) {\n",
    replace:
      "                if false && stepped.insert((from.0, from.1, by)) {\n",
    tests: adapters,
  },
  {
    name: "stepped-all-given",
    breaks:
      "every function of an `I` given as it is takes its dictionary, `doubled(inner, IIterator)`, stepped or not",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    given_as_it_is(tcx, id, ty::ParamTy::new(param.index, param.name))\n",
    replace: "                    true\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_types.rs$"],
    snapshots: true,
  },
  {
    name: "stepped-given-adapter-unseen",
    breaks:
      "`chain(iter)` of an `I` given as it is gives no dictionary for it, and spreads its value as a JS iterator",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                        stepped.insert((caller, param.index, iterator));\n",
    replace: "",
    tests: extensions,
  },
  {
    name: "stepped-asked-extension-unseen",
    breaks:
      "`rev()` of an `I` given as it is, whose `Self: DoubleEndedIterator` it asks, is given no `DoubleEndedIterator` dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                        .filter(|p| p.self_ty().skip_binder() == self_param && is_iterator_extension(tcx, p.def_id()))\n",
    replace: "                        .filter(|_| false)\n",
    tests: extensions,
  },
  {
    name: "stepped-extension-unseen",
    breaks:
      "`next_back(&mut it)` of an `I` given as it is is given no `DoubleEndedIterator` dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                Some(tr) if is_iterator_extension(tcx, tr) => match is_extension_entry(tcx, callee) {\n",
    replace:
      "                Some(tr) if false => match is_extension_entry(tcx, callee) {\n",
    tests: extensions,
  },
  {
    name: "stepped-as-js-iterator-unseen",
    breaks:
      "an `I` given as it is passed to `last<K: DoubleEndedIterator>` is given no dictionary for its back",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                .chain(as_js_iterator(to))\n",
    replace: "",
    tests: extensions,
  },
  {
    name: "stepped-by-every-trait",
    breaks:
      "`back(ends)`, which only steps its `I` from its back, is given its `Iterator` dictionary too",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    true => vec![tr],\n",
    replace: "                    true => vec![tr, iterator],\n",
    tests: ["test/corpus.test.ts", "-t", "^next_back_generic.rs$"],
    snapshots: true,
  },
  {
    name: "stepped-extension-method-no-iterator",
    breaks:
      "`nth_back` of an `I` given as it is, run on a JS iterator made of it, is given no `Iterator` dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    false => vec![tr, iterator],\n",
    replace: "                    false => vec![tr],\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_bound_iterators.rs$"],
  },
  {
    name: "stepped-adapter-unseen",
    breaks:
      "`rfold` of a `map` of an `I` given as it is gives no `DoubleEndedIterator` dictionary for it",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    for &by in by.iter().filter(|&&by| by != iterator) {\n",
    replace: "                    for &by in by.iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_map_rfold.rs$"],
  },
  {
    name: "kept-default-steps-unseen",
    breaks:
      "a blanket impl that keeps a default stepping its `Self` is given no `Iterator` dictionary for its `T`",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    passed.push(((imp, param.index), default));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copied_iterator_defaults.rs$"],
  },
  {
    name: "kept-default-holder-unseen",
    breaks:
      "an impl for `Holder<I>` that keeps a default stepping its `Self` is given no `Iterator` dictionary for `I`",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                            kept_through.push((default, ((imp, param.index), (instance.def_id(), at as u32))));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copied_iterator_defaults.rs$"],
  },
  {
    name: "kept-default-holder-unconditional",
    breaks:
      "an impl for `Holder<I>` whose kept defaults don't step its `Self` is given `I`'s `Iterator` dictionary anyway",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "            if stepped.iter().any(|&(id, index, _)| (id, index) == default) && !passed.contains(&edge) {\n",
    replace: "            if !passed.contains(&edge) {\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_iterator_defaults.rs$"],
    snapshots: true,
  },
  {
    name: "associated-iterator-unstepped",
    breaks:
      "an impl whose associated type is an iterator holding its type parameter isn't given that parameter's dictionary, and a library's is refused",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "            if !iterates || !fns.contains_key(&instance.def_id()) {\n",
    replace:
      "            if true || !iterates || !fns.contains_key(&instance.def_id()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_steps.rs$"],
  },
  {
    name: "associated-iterator-param-unpassed",
    breaks:
      "an impl's `IntoIter` holding its type parameter as it is isn't given that parameter's dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    ty::Param(param) => passed.push(((imp, param.index), (instance.def_id(), at as u32))),\n",
    replace: "                    ty::Param(_) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_steps.rs$"],
  },
  {
    name: "associated-iterator-adapter-unstepped",
    breaks:
      "an impl's associated iterator holding a std adapter of its type parameter, `Fuse<I>`, isn't given that parameter's dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    _ => stepped.extend(held.walk().filter_map(|part| match part.as_type()?.kind() {\n",
    replace:
      "                    _ => stepped.extend(held.walk().filter(|_| false).filter_map(|part| match part.as_type()?.kind() {\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_steps.rs$"],
  },
  {
    name: "dictionary-call-passes-nothing",
    breaks:
      "what a trait's method called through a dictionary is given isn't passed on, so `&mut Self` goes as it is where a JS iterator goes",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    pass_on(&mut passed, callee, args);\n                    continue;\n",
    replace: "                    continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^self_given_on_as_into_iter.rs$"],
  },
  {
    name: "passed-reference-not-peeled",
    breaks:
      "a reference to a parameter, `&mut Self`, isn't passed on as the parameter",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    && let ty::Param(param) = ty.peel_refs().kind()\n                    && !given_by_its_trait",
    replace:
      "                    && let ty::Param(param) = ty.kind()\n                    && !given_by_its_trait",
    tests: ["test/corpus.test.ts", "-t", "^self_given_on_as_into_iter.rs$"],
  },
  {
    name: "passed-as-trait-takes",
    breaks:
      "a trait's own `Self` passed to its method through a dictionary is taken for one made a JS iterator, and a dictionary nothing reads is given",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    && !given_by_its_trait(tcx, callee, index as u32)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_params.rs$"],
    snapshots: true,
  },
  {
    name: "into-iterator-not-iterating",
    breaks:
      "an `IntoIterator` parameter isn't one given a JS iterator, so what's given on to it takes no dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: " || is_std_def(tcx, tr, StdItem::IntoIterator));",
    replace: ");",
    tests: ["test/corpus.test.ts", "-t", "^self_given_on_as_into_iter.rs$"],
  },
  {
    name: "std-held-parameter-not-stepped",
    breaks:
      "a parameter a std body holds in a struct whose impl steps it, `peekable`'s `Self`, isn't stepped, so `@rust-js/std` steps what its callers give as they give it",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "        for held in body_aggregates(tcx, body) {\n",
    replace:
      "        for held in body_aggregates(tcx, body).into_iter().take(0) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "library-supertrait-not-stepped",
    breaks:
      "a parameter given to a library function whose manifest says it steps it by a supertrait of its bound, `Iterator` of an `I: DoubleEndedIterator`, isn't stepped by it, so `rfold` of a held `Fuse` is refused",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                        for by in rustc_middle::ty::elaborate::supertrait_def_ids(tcx, bound.def_id()) {\n",
    replace: "                        for by in [bound.def_id()] {\n",
    tests: ["test/corpus.test.ts", "-t", "^map_stepped_from_its_back.rs$"],
  },
  {
    name: "fused-specialization-not-stepped",
    breaks:
      "a std body's call of a std-private trait's method whose impl the caller's type picks, `FuseImpl::next`, steps nothing, so the impls' methods aren't given `I`'s dictionary",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "            if let Some(spec) = crate::lower::recognition::fact_specialization(tcx, typing_env, callee, args) {\n",
    replace:
      "            if let Some(spec) = crate::lower::recognition::fact_specialization(tcx, typing_env, callee, args).filter(|_| false) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "held-struct-hint-unstepped",
    breaks:
      "what a compiled std struct's `size_hint` steps isn't seen, so std's `rev()` can't be compiled, its `Self` stepped by `Iterator` for its hint",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "            for method in [next, size_hint] {\n",
    replace: "            for method in [next] {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std"],
  },
  {
    name: "compiled-back-hooks-unstepped",
    breaks:
      "what a held compiled iterator's back steps, which `rev()` of it asks, isn't seen, so `I`'s dictionaries aren't given",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                            if (asked || defaulted) && crate::lower::recognition::is_double_ended(tcx, by) {\n",
    replace: "                            if (asked || defaulted) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_enumerate).rs$"],
  },
  {
    name: "held-adapter-bounds-unasked",
    breaks:
      "what a std adapter given to a picked impl asks of its source, `Map<I, F>`'s `I: DoubleEndedIterator` in `FlatMap`'s back, isn't asked, and std's compile refuses stepping `I`",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                        if matches!(held.kind(), ty::Adt(..)) && is_iterator_extension(tcx, bound.def_id()) {\n",
    replace: "                        if false {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "no-iterator-made-unstepped",
    breaks:
      "`step_by` isn't given its `Self`'s dictionary for the `StepBy` it makes an iterator of, so it's refused (ADR 0917)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                .is_some_and(|(callee, _)| crate::lower::recognition::makes_no_iterator(tcx, callee))\n",
    replace:
      "                .is_some_and(|(callee, _)| false && crate::lower::recognition::makes_no_iterator(tcx, callee))\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "extension-default-of-adapter-unstepped",
    breaks: "an extension's own method a compiled adapter of a type parameter writes none of, `self.0.map(f).nth_back(1)`'s, isn't seen to step the parameter by its back, so its function is given no `IDoubleEndedIterator`: the JS throws `it.back is not a function` of the crate's iterator, and std's structs' build refuses it (#811)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                                defaulted = tcx.trait_of_assoc(callee) == Some(by)\n",
    replace: "                                defaulted = false && tcx.trait_of_assoc(callee) == Some(by)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_and_counted_steps_from_their_back.rs$"],
  },
  {
    name: "impl-bounds-unwalked",
    breaks:
      "an `I: Iterator` generic code only passes through a chain of the crate's impls whose associated type bound by `Iterator` it is, `Chain<Nothing<I>, _>: Parser`'s, isn't given its `Iterator`'s dictionary, which the innermost impl's asks: refused, 'stepping `I`, which rust-js found nothing here steps' (#1062)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                        pass_on(&mut passed, held, held_args);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_passed_through_a_chain_of_impls.rs$"],
  },
  {
    name: "impl-bounds-walked-with-bound-lifetimes",
    breaks:
      "an impl's bound of any lifetime, `F: for<'a> Fn(&'a T::Item) -> bool`, is selected with its bound lifetimes left in it, and rustc panics, 'has escaping bound vars' (#1062)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                        let bound = tcx.instantiate_bound_regions_with_erased(bound).trait_ref;\n",
    replace: "                        let bound = bound.skip_binder().trait_ref;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_passed_through_a_chain_of_impls.rs$"],
  },
  {
    name: "item-param-iterator-unstepped",
    breaks: "an impl whose `type Iter = I` is bound by `Iterator` isn't seen to step `I`, so a trait's default it keeps that steps `Self::Iter` `.reduce`s a range given it as if a JS iterator, `TypeError` (#972, ADR 0977)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                if iterates || !extensions.is_empty() {\n                    stepped.insert((imp, param.index, iterator));\n",
    replace: "                if iterates || !extensions.is_empty() {\n",
    tests: ["test/corpus.test.ts", "-t", "^kept_default_steps_assoc.rs$"],
  },
  {
    name: "from-iter-steps-nothing",
    breaks:
      "`Vec::from_iter(w)` of the crate's iterator holding a type parameter `I` steps nothing the analysis sees, so the generic function isn't given `I`'s `Iterator` and is refused, where `w.collect()` compiles (#838)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                Some(given) => (into_iter, tcx.mk_args(&[given.into()])),\n",
    replace: "                Some(_) => (callee, args),\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_of_the_crates_iterator.rs$"],
  },
  {
    name: "std-iterable-param-unstepped",
    breaks:
      "a caller's iterator type parameter given as it is, given to a std function rust-js lowers itself, a `Debug` builder's `entries`, isn't stepped, so its impl takes no `Iterator` dictionary and the crate's own iterator `F: Fn() -> I` makes is read as a JS one, `[[object Iterator]]` (#1147)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                stepped.extend(iterables.map(|(param, _)| (caller, param.index, iterator)));\n",
    replace: "                let _ = iterables;\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_entries_drop_as_std.rs$"],
  },
  {
    name: "std-iterable-of-trait-method-unstepped",
    breaks:
      "a caller's iterator type parameter given as it is, given to std's `Extend::extend`, a trait's method rust-js lowers itself, isn't stepped, so its function takes no `Iterator` dictionary and `extend` reads the crate's own iterator as a JS one (#1147)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "            if is_std_crate(tcx, callee) && foreign.item(callee).is_none() {\n",
    replace: "            if is_std_crate(tcx, callee) && foreign.item(callee).is_none() && tcx.trait_of_assoc(callee).is_none() {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_iterables_given_to_std.rs$"],
  },
  {
    name: "library-impl-dictionary-unstepped",
    breaks:
      "a caller's `I: Iterator + Clone`, given as it is, given to a library's blanket impl's default that hands it on unstepped, `first_of`, isn't stepped though the impl's dictionary it makes steps it by its other defaults: refused, \"stepping `I`\" (#1409)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    } else if foreign.item(imp).is_some() {\n                        steps_through(&mut stepped, imp, impl_args);\n",
    replace: "                    } else if foreign.item(imp).is_some() {\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_blanket_impl_steps_a_callers_parameter$"],
  },
  {
    name: "instance-impl-of-a-callers-param-unpassed",
    breaks:
      "a caller's `I: DoubleEndedIterator + Clone`, given as it is, given to a library's blanket impl the crate compiles itself, isn't stepped by what that impl's defaults step, so it's given no `Iterator` dictionary: refused, \"stepping `I`\" (#1409)",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    if crate::lower::traits::is_lowered_impl(foreign, imp) {\n                        pass_on(&mut passed, imp, impl_args);\n",
    replace: "                    if crate::lower::traits::is_lowered_impl(foreign, imp) && false {\n                        pass_on(&mut passed, imp, impl_args);\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_instance_impl_asks_its_defaults_facts$"],
  },
];
