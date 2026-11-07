// Mutations of src/runtime/dedup_by_cells.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dedup-cells-given-items",
    breaks: "a closure taking `&mut` numbers or text is given them, not cells: `a.value` is `undefined` (#906)",
    file: "src/runtime/dedup_by_cells.js",
    find: "  $dedupBy(v, same, drop, true);\n",
    replace: "  $dedupBy(v, same, drop);\n",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "dedup-cells-drops-none",
    breaks: "`dedup_by` of `Box`es or `Rc`s drops none it takes out (#906)",
    file: "src/runtime/dedup_by_cells.js",
    find: "  $dedupBy(v, same, drop, true);\n",
    replace: "  $dedupBy(v, same, undefined, true);\n",
    tests: ["test/corpus.test.ts", "-t", "^dedup_by_drops_as_it_finds.rs$"],
  },
];
