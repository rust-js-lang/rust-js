// Mutations of crates/rust-js-lower/src/declarations/reach.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-held-blind-to-signatures",
    breaks: "a type an exported function hands JS isn't taken as one JS may hold, so a function value in a field of a type a `pub fn` of a private module returns is called with the field's holder as its `this` (#2024)",
    file: "crates/rust-js-lower/src/declarations/reach.rs",
    find: "                for ty in sig.inputs_and_output {\n                    give(ty);\n                }\n",
    replace: "                let _ = sig;\n",
    tests: ["test/corpus.test.ts", "-t", "^owner_handle_cloned_request.rs$"],
    snapshots: true,
  },
  {
    name: "impl-visibility-asked",
    breaks: "whether JS names an impl, the owner of its type parameters, is asked of its visibility, which a library's impl has none of in its metadata: rustc ICEs, `missing visibility for {impl}`, compiling the app (#2590)",
    file: "crates/rust-js-lower/src/declarations/reach.rs",
    find: "        DefKind::Impl { of_trait: true } => tcx.impl_opt_trait_id(id).is_some_and(|tr| js_names(tcx, tr)),\n",
    replace: "        DefKind::Impl { of_trait: true } => tcx.visibility(id).is_public(),\n",
    tests: ["test/crates-pairs-2.test.ts", "-t", "two crates: library_generic_impls_owning_their_parameters$"],
  },
];
