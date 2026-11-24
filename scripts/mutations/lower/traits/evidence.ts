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
  {
    name: "provided-entry-given-none-of-its-bounds",
    breaks: "a crate's iterator's own provided method's entry is given none of the call's own evidence, `BDefault`, so its impl's function is given what isn't its, or refused (#2421)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "            bounds: &bounds,\n",
    replace: "            bounds: &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_iterator_writing_partition_given_to_generic_code.rs$"],
  },
  {
    name: "provided-entry-bounds-unnormalized",
    breaks: "the call's own bound `B: Extend<Self::Item>` given a crate's iterator's own `partition` isn't of its type's terms, so its entry names its dictionary `BExtendSelfItem` where the impl's function names it `BExtendU32`: needless output, as `evidence_args_given` matches either spelling (#2421)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "                    let tr = (tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr)))\n",
    replace: "                    let tr = (None::<ty::TraitRef<'tcx>>)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_iterator_writing_partition_given_to_generic_code.rs$"],
    snapshots: true,
  },
  {
    name: "provided-entry-never-eta-reduced",
    breaks: "a crate's iterator's own provided method's entry that passes on what it's given is kept an arrow, never the function itself: needless output (#2421)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "        Ok(crate::passed_on::passed_on(&entry, crate::passed_on::Taker::Made, &[]).unwrap_or(entry))\n",
    replace: "        Ok(entry)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_provided_methods_a_type_writes_are_its_own.rs$"],
    snapshots: true,
  },
  {
    name: "provided-entry-own-parameters-fact-unrefused",
    breaks: "an override's own ask of a fact of the method's own type parameter, `size_of::<B>()` of an own `collect`, which the convention doesn't give, isn't refused where its entry is made, so it's asked of what the entry can't name and refused with no word of the override (#2449)",
    file: "crates/rust-js-lower/src/traits/evidence.rs",
    find: "            if of_own(of)\n",
    replace: "            if false && of_own(of)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_own_collect_asking_its_outputs_size_refused.rs$"],
  },
];
