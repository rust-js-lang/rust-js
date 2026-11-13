// Mutations of src/lower/traits/dictionaries/collecting.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "from-iter-entry-folds-to-an-array",
    breaks: "a std collection's `from_iter` given generic code is its own fold into an array, `$foldToArray(items)`, not the collect generic code's own `Vec` collect is, so `collect::<B>()` of `B = Vec<u32>` asks no hint where std's asks it as it grows (#1239)",
    file: "src/lower/traits/dictionaries/collecting.rs",
    find: "            let made = cx.collected_into((Expr::var(\"items\"), None), output, Some(item), Some(param), span);\n",
    replace: "            let made = cx.collected_into((Expr::call_helper(\"$foldToArray\", vec![Expr::var(\"items\")]), None), output, Some(item), None, span);\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_a_type_parameter_is_stds\\.rs$"],
  },
  {
    name: "try-processed-entry-given-no-drop",
    breaks: "a std `Option`'s or `Result`'s `fromIter` entry of a collection whose `from_iter` runs code gives the `GenericShunt` it passes no drop, so what's left of what generic code gave it drops after that `from_iter`, not as its loop ends (#1588)",
    file: "src/lower/traits/dictionaries/collecting.rs",
    find: "            let lent = shunted.then(|| cx.lend_drop_param(\"dropI\".into(), param));\n",
    replace: "            let lent = false.then(|| cx.lend_drop_param(\"dropI\".into(), param));\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter.rs$"],
  },
  {
    name: "collecting-entry-param-unsized",
    breaks: "the type parameter a `fromIter` entry stands for what's given by isn't `Sized`, so `GenericShunt` of it is no iterator and its `IntoIter` has no drop (#1588)",
    file: "src/lower/traits/dictionaries/collecting.rs",
    find: "            ty::TraitRef::new(tcx, sized, [param]).upcast(tcx),\n",
    replace: "            ty::TraitRef::new(tcx, iterator, [param]).upcast(tcx),\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter.rs$"],
  },
];
