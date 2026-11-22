// Mutations of crates/rust-js-lower/src/analysis/cell_borrows/at_hand/local_cells.rs (ADR
// 0093): a local cell its closures share, every use of it one of a closed
// list (#2114).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "local-cell-unaccounted-use-admitted",
    breaks: "a use of a local cell none of the closed list accounts for, the cell handed to a function, is taken as writing nothing, so a map that function may replace is read plainly (#2114)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/at_hand/local_cells.rs",
    find: "                groups[group].writes = None;\n",
    replace: "                let _ = group;\n",
    tests: ["test/at-hand-slots.test.ts", "-t", "a local cell handed to a function"],
  },
  {
    name: "local-cell-clones-unseen",
    breaks: "an `Rc::clone` of a local cell isn't taken as one of its own, so its closures' uses of their clone aren't asked and the cell's first is unaccounted: React's noop renderer's map keeps its memo's borrow checked (#2114)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/at_hand/local_cells.rs",
    find: "                if !is_rc_clone(recognition.tcx, id, generic_args) {\n",
    replace: "                if true {\n",
    tests: ["test/at-hand-slots.test.ts", "-t", "its map a builtin made, reads plainly"],
  },
];
