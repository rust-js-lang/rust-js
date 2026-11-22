// Mutations of crates/rust-js-program/src/js/pattern.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "array-pattern-binds-nothing",
    breaks: "a name an array pattern binds, `const [a, b] = pair`, isn't among what it declares, so JS's scopes take its reads for reads nothing declares: the program refused",
    file: "crates/rust-js-program/src/js/pattern.rs",
    find: "            Pattern::Array(items) => items.iter().flatten().flat_map(Pattern::bound).collect(),\n",
    replace: "            Pattern::Array(_) => Vec::new(),\n",
    tests: ["test/corpus.test.ts", "-t", "^(adapter_size_hints|address_of_a_field_through_a_reference).rs$"],
  },
];
