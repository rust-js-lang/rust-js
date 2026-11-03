// Mutations of src/runtime/cell_replace.js (ADR 0153).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-swap-shared",
    breaks:
      "`Cell::from_mut(&mut a).swap(Cell::from_mut(&mut b))` of two `Vec`s leaves both `b`: each cell's object, written in place, read as itself, not apart (#527)",
    file: "src/runtime/cell_replace.js",
    find: "  [a.value, b.value] = [$apart(b.value), $apart(a.value)];\n",
    replace: "  [a.value, b.value] = [b.value, a.value];\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_cell_(replaced|moved_out).rs$"],
  },
];
