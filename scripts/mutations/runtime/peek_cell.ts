// Mutations of src/runtime/peek_cell.js (ADR 0513).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "peek-cell-lazy-unwritten",
    breaks: "`peek_mut` of a lazy `Peekable` writes a copy, not the item it gives next",
    file: "src/runtime/peek_cell.js",
    find: "        step.value = value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_and_peek_mut.rs$"],
  },
];
