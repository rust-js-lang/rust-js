// Mutations of src/lower/declarations/reach.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-held-blind-to-signatures",
    breaks: "a type an exported function hands JS isn't taken as one JS may hold, so a function value in a field of a type a `pub fn` of a private module returns is called with the field's holder as its `this` (#2024)",
    file: "src/lower/declarations/reach.rs",
    find: "                for ty in sig.inputs_and_output {\n                    give(ty);\n                }\n",
    replace: "                let _ = sig;\n",
    tests: ["test/corpus.test.ts", "-t", "^owner_handle_cloned_request.rs$"],
    snapshots: true,
  },
];
