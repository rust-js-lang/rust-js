// Mutations of src/lower/analysis/chars_asked.rs (ADR 1117).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chars-crossing-unseen",
    breaks:
      "a library whose public function gives out a `Chars` makes it JS's own string iterator, whose back, rest and hint the crate it's given to can't ask",
    file: "src/lower/analysis/chars_asked.rs",
    find: "        let crosses = library && {\n",
    replace: "        let crosses = false && {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_chars_given"],
  },
  {
    name: "chars-dyn-unseen",
    breaks: "a `Chars` made a `dyn DoubleEndedIterator` is JS's own string iterator, whose `next_back()` its dictionary can't step",
    file: "src/lower/analysis/chars_asked.rs",
    find: "                } => recognition.holds_str_chars(thir[source].ty),\n",
    replace: "                } => false && recognition.holds_str_chars(thir[source].ty),\n",
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "instance-chars-unasked",
    breaks:
      "a `Chars`' rest a library's generic function the consumer compiles itself asks, `chars.as_str()`, isn't seen, so it's JS's own string iterator, and `$restStr` of it throws (#1325)",
    file: "src/lower/analysis/chars_asked.rs",
    find: "            || mir_asks\n",
    replace: "            || mir_asks && false\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_instances_ask_what_the_crate_doesnt"],
  },
  {
    name: "vec-grown-by-chars-unasked",
    breaks: "a `Vec` grown by a string's `chars()` in a crate that reads capacities doesn't make it `$chars`, whose hint is std's, so it's refused where nothing else asks its hint (#1316)",
    file: "src/lower/analysis/chars_asked.rs",
    find: "                        || keeps_capacity\n",
    replace: "                        || false\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_grown_by_iterators_whose_hint_is_stds.rs$"],
  },
];
