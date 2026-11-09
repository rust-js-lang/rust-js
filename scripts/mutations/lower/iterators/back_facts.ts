// Mutations of src/lower/iterators/back_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "filter-stepped-by-its-type",
    breaks:
      "a filter's question of whether the crate steps it from its back is its type's in back_stepped's set, so one only folded forward of a type stepped from its back elsewhere is `$filterEnds` (#851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "            || r::filtered_source(self.tcx, ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_folded_and_rfolded.rs$"],
    snapshots: true,
  },

  {
    name: "made-from-unread",
    breaks: "what a call is given isn't asked, only what it makes, so a map made in place where it's stepped, `count().map(f).rfold(..)`, built at its step, is never made knowing its back, and is refused (#851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "                && (self.made_into.iter().chain(&self.made_from)).any(|&(local, made)| {\n",
    replace: "                && (self.made_into.iter()).any(|&(local, made)| {\n",
    tests: ["test/corpus.test.ts", "-t", "^backs_given_behind_impls.rs$"],
    snapshots: true,
  },
  {
    name: "bare-array-iterator-trusted",
    breaks: "a variable of an array's iterator's type alone is taken to have a back, though it may hold the array, so a chain of one `w` is `$lazyChain(.., w)`, which throws `it.back is not a function` (#851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "                    && (wrapped || !self.steps_an_array(ty))\n",
    replace: "                    && true\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_chain.rs$"],
  },
  {
    name: "compiled-variable-trusted",
    breaks: "a variable holding std's compiled struct is taken to have a back by its type, though it has one only given it, so a `rev()` of what a function returns throws `it.back is not a function` (#856, #851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "                    && !self.compiled_struct(ty, value)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^returned_impl_ends.rs$"],
  },
  {
    name: "chain-stepped-by-its-type",
    breaks: "a chain's question of whether the crate steps it from its back is its type's in back_stepped's set, so one only stepped forward, of a type stepped from its back elsewhere, is made knowing its back where JS's own form reads (#851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "            || r::chained_sources(self.tcx, ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(held_beneath_adapters_from_its_back|cloned_of_a_chain_stepped_from_its_back|chain_stepped_from_its_back_elsewhere|chain_under_a_held_map_from_its_back|chain_of_adapters_stepped_from_its_back|chain_sides_own_steps|rev_chain|array_cloned_stepped_from_its_back|cloned_items_from_both_ends|copied_given_to_generic_backs|copied_skip_take_and_filter_stepped_from_back|adapters_given_to_generic_backs|map_ends_size_hint|back_folds_of_sources_made_knowing_their_back|consumer_drops_what_is_left|adapters_own_work_that_panics).rs$"],
    snapshots: true,
  },

];
