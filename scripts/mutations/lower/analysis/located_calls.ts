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
  {
    name: "no-callers-no-location",
    breaks: "a body no seen caller names, a closure std's MIR inlined, `clone_try_fold`'s, gives its dictionary's clone no location, so compiled std's `Cloned::try_fold` names `<unknown>` where Rust names `cloned.rs:29:32` (#2522)",
    file: "crates/rust-js-lower/src/analysis/located_calls.rs",
    find: "        let Some(instances) = seen.listed_or_any().filter(|instances| !instances.is_empty()) else {\n",
    replace: "        let Some(instances) = seen.listed_or_any() else {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
];
