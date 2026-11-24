// Mutations of crates/rust-js-lower/src/analysis/located_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "gives-location-never",
    breaks: "generic code's call through a dictionary of a method whose model raises at its caller never gives its entry the location, so a `RefCell`'s clone, an integer's `/`, raise at `<unknown>` or where the dictionary was made, not the generic line (#2324, #2457)",
    file: "crates/rust-js-lower/src/analysis/located_calls.rs",
    find: "        if !crate::recognition::model_raises_at_caller(tcx, callee) {\n",
    replace: "        if true {\n",
    tests: ["test/binaries.test.ts", "-t", "dictionary"],
  },
];
