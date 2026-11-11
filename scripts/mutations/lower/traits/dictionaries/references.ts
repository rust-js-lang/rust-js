// Mutations of src/lower/traits/dictionaries/references.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "through-box-entries-unnamed",
    breaks: "std's impl that passes a trait on from a `&mut T` to `T` gives `T`'s entries the box as it is, `show(&mut parser)` showing `undefined` (#1414)",
    file: "src/lower/traits/dictionaries/references.rs",
    find: "            if given.iter().any(|&kind| kind != 0) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "reference-not-passed-on",
    breaks: "a reference's dictionary of std's impl that passes a trait on to what it refers to, `&Parser: Display`, is refused",
    file: "src/lower/traits/dictionaries/references.rs",
    find: "        passes_on.then(|| {\n",
    replace: "        false.then(|| {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
];
