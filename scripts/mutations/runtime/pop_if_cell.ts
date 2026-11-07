// Mutations of src/runtime/pop_if_cell.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pop-if-unwritten",
    breaks: "what the closure changes of the last item isn't kept",
    file: "src/runtime/pop_if_cell.js",
    find: "    v[v.length - 1] = last.value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "pop-if-unwritten-as-it-unwinds",
    breaks: "what a `pop_if` closure wrote through its `&mut` before it panicked isn't kept, so what it replaced, dropped already, is dropped again (#906)",
    file: "src/runtime/pop_if_cell.js",
    find: "  try {\n    popped = holds(last);\n  } finally {\n    v[v.length - 1] = last.value;\n  }\n",
    replace: "  popped = holds(last);\n  v[v.length - 1] = last.value;\n",
    tests: ["test/corpus.test.ts", "-t", "^dedup_by_drops_as_it_finds.rs$"],
  },
];
