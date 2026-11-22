// Mutations of crates/rust-js-lower/src/traits/evidence.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-dictionary-any-step",
    breaks:
      "a call leaves out a dictionary at its end whose entry is the default's but whose step isn't, a range's `{ next: $rangeNext }`, so the callee steps the range as a JS iterator (ADR 0995)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "if matches!(props.as_slice(), [js::Prop::Field(name, value)] if name == entry && value.helper_name() == Some(helper)))",
    replace:
      'if matches!(props.as_slice(), [js::Prop::Field(name, value)] if name == entry && value.helper_name().is_some() && helper != ""))',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_map|cloned_counted_adapters|associated_iterator_steps)\\.rs$",
    ],
  },
  {
    name: "own-facts-through-a-library-dictionary-reanalyzed",
    breaks: "a call through a library's exported impl's dictionary gives the facts the consumer's analysis of the default's body finds, one more than the library's JS takes, so a size lands on its `OneShot` (#1612)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "        let facts = match (self.exported_impl(tr)).and_then(|_| self.krate.foreign.trait_method_facts(id)) {\n",
    replace: "        let facts = match (self.exported_impl(tr)).and_then(|_| self.krate.foreign.trait_method_facts(id)).filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "library_default_where_clause_through_its_dictionary"],
  },
  {
    name: "facts-through-a-type-parameters-dictionary-unsaid",
    breaks: "a call of a library trait's generic method through a type parameter's dictionary gives none of the facts the library's contract lists, so its entry's `size_of::<X>()` reads `undefined`, `0` where native Rust's is `7` (#2285)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "    let Some(contract) = foreign.trait_method_facts(id) else {\n",
    replace: "    let Some(contract) = foreign.trait_method_facts(id).filter(|_| false) else {\n",
    tests: ["test/crates.test.ts", "-t", "library_trait_method_fact_through_a_dictionary"],
  },
];
