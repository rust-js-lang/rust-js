// Mutations of crates/rust-js-lower/src/traits/dictionaries/references.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "through-box-entries-unnamed",
    breaks: "std's impl that passes a trait on from a `&mut T` to `T` gives `T`'s entries the box as it is, `show(&mut parser)` showing `undefined` (#1414)",
    file: "crates/rust-js-lower/src/traits/dictionaries/references.rs",
    find: "            if given.iter().any(|&kind| kind != 0) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "reference-not-passed-on",
    breaks: "a reference's dictionary of std's impl that passes a trait on to what it refers to, `&Parser: Display`, is refused",
    file: "crates/rust-js-lower/src/traits/dictionaries/references.rs",
    find: "        passes_on.then(|| {\n",
    replace: "        false.then(|| {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "through-box-words-made-first",
    breaks: "a dictionary through a reference makes its refusal words before it's known to refuse, printing the trait's path and its type in every compile that passes one, a walk of every crate's modules, about 37 ms (#2321)",
    file: "crates/rust-js-lower/src/traits/dictionaries/references.rs",
    find: "                tr.self_ty()\n            )\n        };\n",
    replace: "                tr.self_ty()\n            )\n        };\n        let _ = what();\n",
    tests: ["test/phase-times.test.ts", "-t", "a lowering forces no walk of every crate"],
  },
];
