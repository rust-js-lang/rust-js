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
];
