// Mutations of crates/rust-js-lower/src/analysis/iterator_facts/typed_key.rs (ADR 0093):
// which item a type's question is asked in (#2133).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "typed-key-of-every-item",
    breaks:
      "a type of type parameters is keyed crate-wide, as one of none is, so two functions whose parameters are spelled alike share its facts: `held`'s `Enumerate<Box<I>>`, std's struct, makes `folded`'s one too, its source made without its back, `enumerated.iter.back is not a function` (#2133)",
    file: "crates/rust-js-lower/src/analysis/iterator_facts/typed_key.rs",
    find: "    (of_params.then(|| tcx.typeck_root_def_id(owner)), ty)\n",
    replace: "    (of_params.then(|| tcx.typeck_root_def_id(owner)).filter(|_| false), ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^type_parameters_spelled_alike_keep_their_own_facts.rs$"],
    snapshots: true,
  },
];
