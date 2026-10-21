// Mutations of src/runtime/iterator.js (ADRs 0061, 0421).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iterator-of-forgotten",
    breaks: "a JS iterator of the crate's iterator doesn't say what it's of, which its drop drops",
    file: "src/runtime/iterator.js",
    find: "  it.of = iterator;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
  },
  {
    name: "iterator-back-forgotten",
    breaks: "a range or the crate's iterator given as a double-ended one has no `back()`",
    file: "src/runtime/iterator.js",
    find: "    it.back = () => step(nextBack(iterator));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "knowing-installs-nothing",
    breaks: "a use site's back and length aren't given the `$iterator` a held adapter steps",
    file: "src/runtime/iterator.js",
    find: "    if (back !== undefined) it.back = () => step(back(iterator));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
];
