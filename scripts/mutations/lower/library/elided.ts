// Mutations of src/lower/library/elided.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "elided-not-closed-over-calls",
    breaks: "an exported function's contract says only what its own body leaves out, so `Board::top`, whose private `top_of` copies, is its library's, and the consumer's change of the tally changes the board's (#2112's change D)",
    file: "src/lower/library/elided.rs",
    find: "            work.extend(callees.get(&at).into_iter().flatten().copied());\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_elides_through_a_private_callee"],
  },
  {
    name: "elided-impl-methods-dropped",
    breaks: "an impl's contract says nothing of what its methods leave out, so `held.first()` of `impl First for Spots` is its library's, whose copy of the `Spot` is left out, and the consumer's change of it changes `held`'s (#2112's change D)",
    file: "src/lower/library/elided.rs",
    find: "            all.entry(imp).or_default().extend(found);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "elided-trait-method-impls-unfollowed",
    breaks: "a function's contract says nothing its trait method calls leave out through the crate's impls, so `first_of`'s `spots.first()` is its library's, whose copy is left out (#2112's change D)",
    file: "src/lower/library/elided.rs",
    find: "            if let Some(trait_id) = tcx.trait_of_assoc(at) {\n",
    replace: "            if let Some(trait_id) = tcx.trait_of_assoc(at).filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
];
