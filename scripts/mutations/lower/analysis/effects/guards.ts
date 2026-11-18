// Mutations of src/lower/analysis/effects/guards.rs, what a held `RefCell` guard says of its cell (#2112 B).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/effects/guards.rs";

export const mutations: Mutation[] = [
  {
    name: "effects-guards-cover-nothing",
    breaks:
      "a `RefCell` guard held across code covers nothing of its cell, so `f(items[i])` under `take_first`'s `borrow_mut()` forgets the guard's length and `items.remove(i)` is checked again (the lossy direction)",
    file,
    find: "            if !covered.is_empty() {\n",
    replace: "            if false {\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "guard keeps its cell's length until it drops"],
  },
  {
    name: "effects-guard-outlives-its-drop",
    breaks:
      "a guard is still held past its drop, so code that clears the cell through another `Rc` once it's dropped keeps its length, and `after_a_dropped_guard`'s index reads past the end where Rust panics (the unsound direction)",
    file,
    find: "    for local in ended {\n        held.shift_remove(&local);\n    }\n",
    replace: "    let _ = (ended, held);\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_written_through_a_clone_under_its_guard.rs$"],
  },
];
