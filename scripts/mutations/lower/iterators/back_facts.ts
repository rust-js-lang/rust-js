// Mutations of src/lower/iterators/back_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "filter-stepped-by-its-type",
    breaks:
      "a filter's question of whether the crate steps it from its back is its type's in back_stepped's set, so one only folded forward of a type stepped from its back elsewhere is `$filterEnds` (#851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "            || crate::lower::recognition::filtered_source(self.tcx, ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_folded_and_rfolded.rs$"],
    snapshots: true,
  },
  {
    name: "map-stepped-by-its-type",
    breaks: "a map's question of whether the crate steps it from its back is its type's in back_stepped's set, not its value's, so a map only folded forward, of a type folded from its back elsewhere, is `$mapEnds` where JS's own `.map` reads (#851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "        if !self.back_by_value(ty) || self.by_set.get() {\n",
    replace: "        if true {\n",
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
    breaks: "a variable holding std's compiled struct is taken to have a back by its type, though it has one only given it, so a `flat_map`'s `rev()` throws `it.back is not a function` (#856, #851)",
    file: "src/lower/iterators/back_facts.rs",
    find: "                    && !self.compiled_struct(self.reveal(ty), value)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flatten_from_its_back.rs$"],
  },
];
