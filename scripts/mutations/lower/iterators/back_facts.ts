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

  {
    name: "char-indices-back-unknown",
    breaks: "a `char_indices()` isn't asked whether it knows its back as a `chars()` is, so generic code's `nth_back` of one is refused",
    file: "src/lower/iterators/back_facts.rs",
    find: "        (text.is_str_chars(ty) || text.is_char_indices(ty)).then(|| self.chars_asked_whole(ty))\n",
    replace: "        text.is_str_chars(ty).then(|| self.chars_asked_whole(ty))\n",
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "chars-cloned-unseen",
    breaks: "a `chars()` the crate clones is JS's own string iterator, which has no `clone()`",
    file: "src/lower/iterators/back_facts.rs",
    find: "        self.krate.chars_whole\n            || self\n",
    replace: "        self.krate.chars_whole\n            || false && self\n",
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "back-refusal-names-no-follow-up",
    breaks: "a refusal to step from its back what holds std's struct of an adapter a function gives doesn't say so, nor its follow-up (#1235)",
    file: "src/lower/iterators/back_facts.rs",
    find: "            true => \", which holds std's struct of an adapter a function gives (#1235)\",\n",
    replace: "            true => \"\",\n",
    tests: ["test/diagnostics.test.ts", "-t", "a flatten of a function's impl stepped from its back"],
  },
  {
    name: "knowing-in-its-maker-unread",
    breaks: "a `map` generic code makes of its own type parameter's iterator, where its hint is asked, is JS's own, `(0, None)` (#1240)",
    file: "src/lower/iterators/back_facts.rs",
    find: "            || (self.krate.hinted.knowing_in).contains(&(self.tcx.typeck_root_def_id(self.frame.owner), ty))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
  {
    name: "counted-in-its-maker-unread",
    breaks: "a `take(2)` generic code makes of its own type parameter's iterator, where its hint is asked, is JS's own `.take(2)`, `(0, None)` (#1240)",
    file: "src/lower/iterators/back_facts.rs",
    find: " || self.krate.hinted.counted_in.contains(&(owner, ty))",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
];
