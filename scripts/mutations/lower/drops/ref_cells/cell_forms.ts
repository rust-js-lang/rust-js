// Mutations of crates/rust-js-lower/src/drops/ref_cells/cell_forms.rs (ADR 0093;
// #2490: `CellForm`'s bridge from a body being lowered).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "walked-params-drop-nothing",
    breaks:
      "a type parameter a function is given a drop for drops nothing where its cells' borrows are counted, so a guard held while `let _it = it;` drops a `T` whose destructor asks of the cell isn't counted, and its `try_borrow()` is `Ok` (a wrong answer, #1321)",
    file: "crates/rust-js-lower/src/drops/ref_cells/cell_forms.rs",
    find: "            state.give(crate::traits::drop_slot(self.tcx, self.slots(), root, index));\n",
    replace: "                _ = index;\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_held_while_a_type_parameters_value_drops.rs$"],
  },
];
