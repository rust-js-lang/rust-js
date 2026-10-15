// Mutations of src/lower/analysis/iterator_steps.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const adapters = ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"];

export const mutations: Mutation[] = [
  {
    name: "stepped-param-unseen",
    breaks: "an adapter's `next`, which steps its `I`, is given no dictionary for it",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    stepped.insert((caller, param.index));\n",
    replace: "",
    tests: adapters,
  },
  {
    name: "stepped-through-impl-unseen",
    breaks: "`skip_one`, which steps a `Doubled<I>` by its impl's `next`, is given no dictionary for `I`",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    pass_on(&mut passed, instance.def_id(), instance.args);\n                }\n                continue;\n",
    replace: "                }\n                continue;\n",
    tests: adapters,
  },
  {
    name: "stepped-passed-on-unseen",
    breaks: "a function that passes its `I` to one that steps it is given no dictionary for it",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "            if stepped.contains(&to) && stepped.insert(from) {\n",
    replace: "            if false && stepped.contains(&to) && stepped.insert(from) {\n",
    tests: adapters,
  },
  {
    name: "stepped-all-given",
    breaks: "every function of an `I` given as it is takes its dictionary, `doubled(inner, IIterator)`, stepped or not",
    file: "src/lower/analysis/iterator_steps.rs",
    find: "                    given_as_it_is(tcx, id, ty::ParamTy::new(param.index, param.name))\n",
    replace: "                    true\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_types.rs$"],
    snapshots: true,
  },
];
