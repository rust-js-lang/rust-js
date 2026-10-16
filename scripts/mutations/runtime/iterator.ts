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
];
