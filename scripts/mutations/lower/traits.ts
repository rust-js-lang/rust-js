// Mutations of src/lower/traits.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "projection-extension-ungiven",
    breaks:
      "an extension's bound on an associated type, `U::IntoIter: DoubleEndedIterator` of std's `FlatMap`'s back, is given no dictionary, so its evidence is the `IntoIterator`'s `IntoIterIterator()`, an `Iterator`'s: `it.back is not a function` (ADR 0936)",
    file: "src/lower/traits.rs",
    find: "        && matches!(alias.self_ty().kind(), ty::Param(_))\n",
    replace:
      "        && matches!(alias.self_ty().kind(), ty::Param(_))\n        && false\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^associated_double_ended.rs$"],
  },
  {
    name: "supertrait-key-instantiated",
    breaks:
      "`Pair<u32, String>: Label<A> + Label<B>`'s impl writes its supertraits as `LabelU32` and `LabelString`, and generic code reads `LabelA` and `LabelB`",
    file: "src/lower/traits.rs",
    find: "                (true, false) => trait_word(tcx, declared),\n",
    replace:
      "                (true, false) => trait_word(tcx, instantiated),\n",
    tests: ["test/corpus.test.ts", "-t", "generic_traits"],
  },
  {
    name: "supertrait-key-trait-only",
    breaks:
      "`trait Both: Label<u32> + Label<String>` has two supertraits keyed `Label`, which collide",
    file: "src/lower/traits.rs",
    find: "                (true, false) => trait_word(tcx, declared),\n",
    replace:
      "                (true, false) => tcx.item_name(declared.def_id).to_string(),\n",
    tests: ["test/corpus.test.ts", "-t", "generic_traits"],
  },
  {
    name: "supertrait-bound-regions",
    breaks:
      "a `for<'a> Greet<&'a str>` supertrait keeps its bound lifetime, and rustc's trait selection panics on it",
    file: "src/lower/traits.rs",
    find: "            let ty::ClauseKind::Trait(instantiated) = tcx.instantiate_bound_regions_with_erased(instantiated.kind())\n            else {\n                return None;\n            };\n            Some((declared.trait_ref, instantiated.trait_ref, span))\n",
    replace:
      "            let ty::ClauseKind::Trait(instantiated) = instantiated.kind().skip_binder()\n            else {\n                return None;\n            };\n            Some((declared.trait_ref, instantiated.trait_ref, span))\n",
    tests: ["test/corpus.test.ts", "-t", "generic_traits"],
  },
  {
    name: "item-bounds-of-gats",
    breaks:
      "a generic associated type's bounds are instantiated with the trait's arguments alone, and rustc panics",
    file: "src/lower/traits.rs",
    find: "        let item_args = ty::GenericArgs::for_item(tcx, item.def_id, |param, _| match args.get(param.index as usize) {\n            Some(&arg) => arg,\n            None => tcx.lifetimes.re_erased.into(),\n        });",
    replace: "        let item_args = args;",
    tests: ["test/corpus.test.ts", "-t", "generic_associated_types"],
  },
  {
    name: "gat-bounds-left-out",
    breaks:
      "a generic associated type's bound, `type Item<'a>: Debug`, isn't in its trait's dictionary",
    file: "src/lower/traits.rs",
    find: "        if own\n            .iter()\n            .any(|param| !matches!(param.kind, ty::GenericParamDefKind::Lifetime))",
    replace: "        if !own.is_empty()",
    tests: ["test/corpus.test.ts", "-t", "generic_associated_types"],
  },
  {
    name: "gat-bound-sized-counted",
    breaks:
      "a generic associated type of a const parameter, whose only bound is `Sized`, is taken to have one",
    file: "src/lower/traits.rs",
    find: "                ty::ClauseKind::Trait(bound) => tcx.is_lang_item(bound.def_id(), LangItem::Sized),",
    replace: "                ty::ClauseKind::Trait(_) => false,",
    tests: ["test/corpus.test.ts", "-t", "generic_associated_types"],
  },
  {
    name: "supertraits-by-reference-alike",
    breaks:
      "`Add<Base> + for<'r> Add<&'r Base>` as supertraits share one name, `AddBase`, which is an error",
    file: "src/lower/traits.rs",
    find: "                (true, true) => trait_word_with_refs(tcx, declared, true),",
    replace: "                (true, true) => trait_word(tcx, declared),",
    tests: ["test/corpus.test.ts", "-t", "supertraits_by_reference"],
  },
  {
    name: "component-takes-dictionary",
    breaks:
      "a component bound by `Default` takes a dictionary, `Card(props, CDefault)`, which React never gives",
    file: "src/lower/traits.rs",
    find: "    if bindings::is_component(tcx, id) {\n        return result;\n    }",
    replace: "",
    tests: ["test/jsx-props.test.ts", "-t", "and the component no dictionary"],
  },
  {
    name: "marker-bound-takes-dictionary",
    breaks:
      "a bound of a trait with nothing in it takes a dictionary, `named(value, TMarker)`, which its `.d.ts` doesn't say",
    file: "src/lower/traits.rs",
    find: "    (has_dictionary_of(tcx, foreign, tr.def_id, tr.self_ty()) && !is_marker(tcx, foreign, tr.def_id)).then_some(tr)",
    replace:
      "    has_dictionary_of(tcx, foreign, tr.def_id, tr.self_ty()).then_some(tr)",
    tests: ["test/traits.test.ts", "-t", "nothing in it passes no dictionary"],
  },
  {
    name: "marker-copy-supertrait",
    breaks:
      "a trait of no items whose supertrait is `Copy` passes no dictionary, and generic code has no `Copy` to copy by",
    file: "src/lower/traits.rs",
    find: "            .all(|(_, tr, _)| !operational(tcx, foreign, tr.def_id) || is_marker(tcx, foreign, tr.def_id))",
    replace: "            .all(|(_, _, _)| true)",
    tests: ["test/traits.test.ts", "-t", "nothing in it passes no dictionary"],
  },
  {
    name: "copy-bound-always-dropped",
    breaks:
      "a `Copy` bound never takes a copy function, so `kept(p)` shares `p`'s object",
    file: "src/lower/traits.rs",
    find: "        && !given.copied.contains(&(tcx.typeck_root_def_id(id), param.index))\n",
    replace: "",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a Copy bound takes a copy only",
    ],
  },
  {
    name: "copy-bound-always-kept",
    breaks:
      "every `Copy` bound takes a copy function, so `twice(name)` is given `{ copy: (value) => value }`",
    file: "src/lower/traits.rs",
    find: "    if tcx.is_lang_item(tr.def_id, LangItem::Copy)\n        && super::analysis::copies_by_callers(tcx, foreign, id)",
    replace:
      "    if false\n        && super::analysis::copies_by_callers(tcx, foreign, id)",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a Copy bound takes a copy only",
    ],
  },
  {
    name: "binding-trait-dictionary",
    breaks:
      "a bound of a trait of bindings takes a dictionary, `isReactRuleError(e, lintErrorRuleIdentified())`",
    file: "src/lower/traits.rs",
    find: "        && (tcx.associated_items(id).in_definition_order()).all(|item| bindings::is_binding(tcx, item.def_id))\n",
    replace:
      "        && tcx.associated_items(id).in_definition_order().next().is_none()\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "binding method is read of the value",
    ],
  },
  {
    name: "iterator-bound-not-evidence",
    breaks:
      "an adapter's `I: Iterator` takes no dictionary, so its `next` has none to step `I` by",
    file: "src/lower/traits.rs",
    find: "given.stepped.contains(&(id, param.index, tr.def_id))",
    replace: "false",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "into-iter-ref-bound-undictionaried",
    breaks:
      "a `for<'a> &'a mut C: IntoIterator` bound is given no dictionary, and `for x in c` of it is refused",
    file: "src/lower/traits.rs",
    find: "        && (!is_std_def(tcx, trait_id, StdItem::IntoIterator) || into_iter_by_dictionary(tcx, trait_id, self_ty))\n",
    replace: "        && !is_std_def(tcx, trait_id, StdItem::IntoIterator)\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_ref_param.rs$"],
  },
  {
    name: "into-iter-param-bound-dictionaried",
    breaks:
      "an `I: IntoIterator` bound is given a dictionary none passes, where it's given the JS iterator, and its `IntoIter`'s drop is lost",
    file: "src/lower/traits.rs",
    find: "        && (!is_std_def(tcx, trait_id, StdItem::IntoIterator) || into_iter_by_dictionary(tcx, trait_id, self_ty))\n",
    replace:
      "        && (true || into_iter_by_dictionary(tcx, trait_id, self_ty))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_method_drops|std_projection_drops).rs$",
    ],
  },
  {
    name: "extension-bound-undictionaried",
    breaks:
      "an `I` given as it is is given no `DoubleEndedIterator` or `ExactSizeIterator` dictionary, and its `len()` is refused",
    file: "src/lower/traits.rs",
    find: "    if (is_std_def(tcx, tr.def_id, StdItem::Iterator) || super::recognition::is_iterator_extension(tcx, tr.def_id))\n",
    replace: "    if is_std_def(tcx, tr.def_id, StdItem::Iterator)\n",
    tests: ["test/corpus.test.ts", "-t", "^given_iterator_extensions.rs$"],
  },
  {
    name: "extension-implies-no-iterator",
    breaks:
      "an `I: DoubleEndedIterator` given as it is is given no `Iterator` dictionary, and its `next()` is refused",
    file: "src/lower/traits.rs",
    find: "        if !super::recognition::implies_iterator(tcx, tr.def_id) {\n            return None;\n        }\n        let iterator",
    replace:
      "        if true {\n            return None;\n        }\n        let iterator",
    tests: ["test/corpus.test.ts", "-t", "^extension_bound_iterators.rs$"],
  },
  {
    name: "extension-iterator-ungiven",
    breaks:
      "a type parameter given as it is bound by the crate's trait that extends `Iterator` is given no `Iterator` dictionary, and stepping it asks its trait's",
    file: "src/lower/traits.rs",
    find: "        if !super::recognition::implies_iterator(tcx, tr.def_id) {\n            return None;\n        }\n        let iterator = ty::TraitRef",
    replace:
      "        if !super::recognition::is_iterator_extension(tcx, tr.def_id) {\n            return None;\n        }\n        let iterator = ty::TraitRef",
    tests: ["test/corpus.test.ts", "-t", "^extension_trait_params.rs$"],
  },
  {
    name: "index-bound-as-itself",
    breaks:
      "a bound of `Index<J>` on a `Vec<T>` isn't its `J: SliceIndex<[T]>`, whose dictionary indexes",
    file: "src/lower/traits.rs",
    find: "    if let Some(slice_index) = super::recognition::index_by_slice_index(tcx, tr) {",
    replace:
      "    if let Some(slice_index) = super::recognition::index_by_slice_index(tcx, tr).filter(|_| false) {",
    tests: ["test/corpus.test.ts", "-t", "^index_bound_generic.rs$"],
  },
  {
    name: "library-steps-unread",
    breaks:
      "a library's manifest isn't asked which type parameters it steps, and its consumer gives no dictionary",
    file: "src/lower/traits.rs",
    find: "|| foreign.steps(id, param.index, tr.def_id);\n",
    replace: ";\n",
    tests: ["test/crates.test.ts", "-t", "two crates: extension_default"],
  },
  {
    name: "into-iter-held-no-dictionary",
    breaks:
      "an `IntoIterator` whose `IntoIter` is held as it is is given no dictionary",
    file: "src/lower/traits.rs",
    find: "        && super::iterators::into_iter_held_as_it_is(tcx, id, tr.self_ty())\n    {\n        return Some(tr);",
    replace: "        && false\n    {\n        return Some(tr);",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "library-copy-bound-recomputed",
    breaks:
      "a consumer gives a library's export a `TCopy` its contract says it takes none of, `pair(1, [2], undefined, wrapDisplay)`, which puts `undefined` where its `UDisplay` is (ADR 0914, amended)",
    file: "src/lower/traits.rs",
    find: "        && let Some(item) = foreign.item(id)\n        && !(item.calling.param(param.index))",
    replace:
      "        && let Some(item) = foreign.item(id).filter(|_| false)\n        && !(item.calling.param(param.index))",
    tests: ["test/cross-crate-items.test.ts", "-t", "contract says it takes"],
  },
  {
    name: "clone-bound-always-given",
    breaks:
      "a `T: Clone` only the crate's callers give, none what isn't its own clone, still takes its dictionary, `readContext(context, TClone)` (ADR 0914, amended)",
    file: "src/lower/traits.rs",
    find: "            true => super::analysis::clones_by_itself(tcx, foreign, given.clones, (id, param.index)),\n",
    replace: "            true => false,\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "judged by what its callers give",
    ],
  },
  {
    name: "library-clone-bound-recomputed",
    breaks:
      "a consumer gives a library's export a `TClone` its contract says it takes none of, `pairClone(1, [3], undefined, wrapDisplay)`, which puts `undefined` where its `UDisplay` is (ADR 0914, amended)",
    file: "src/lower/traits.rs",
    find: "                !(item.calling.param(param.index)).is_some_and(|param| param.has(crate::library::ParamFact::Clone))\n",
    replace: "                false\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "contract says it takes"],
  },
];
