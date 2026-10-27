// Mutations of src/runtime/fold_by.js (#162): a std struct's fold, or a
// value given as it is, not through the crate's own `fold`, or dropping
// what it shouldn't.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rev-advance-by-stepped",
    breaks: "a compiled `Rev`'s `advance_by` of a `Vec`'s items steps each from the back, not its `advance_back_by`'s one slice, front to back (#286)",
    file: "src/runtime/fold_by.js",
    find: "  if (it.items !== undefined && drop !== undefined) return $advanceSlice(it, m, drop, true);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
];
