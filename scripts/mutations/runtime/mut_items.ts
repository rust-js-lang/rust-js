// Mutations of src/runtime/mut_items.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "item-pointer-of-the-handle",
    breaks: "a pointer to an item `iter_mut()` lends is the handle, not its slot's, so a read through it as another type's bytes finds none (#1283)",
    file: "src/runtime/mut_items.js",
    find: "  return $ptr(handle[$itemArray], handle[$itemIndex]);\n",
    replace: "  return handle;\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_to_a_lent_slot.rs$"],
  },
];
