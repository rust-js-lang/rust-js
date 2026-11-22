// Mutations of crates/rust-js-lower/src/analysis/revealed.rs, what each impl Trait of a body hides.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "trait-impl-trait-unrevealed",
    breaks:
      "a trait method's `impl Trait` result stays its trait's projection where it's called on its type, so a `rev()` of its impl's `Map` takes it for one that knows its back, and throws `it.back is not a function` (#2022)",
    file: "crates/rust-js-lower/src/analysis/revealed.rs",
    find: "if self.tcx.is_impl_trait_in_trait(def_id) && !crate::generic_facts::may_be_any_type(ty)",
    replace: "if false && self.tcx.is_impl_trait_in_trait(def_id) && !crate::generic_facts::may_be_any_type(ty)",
    tests: ["test/corpus.test.ts", "-t", "^trait_method_impl_double_ended_result.rs$"],
  },
];
