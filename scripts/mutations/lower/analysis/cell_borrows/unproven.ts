// Mutations of crates/rust-js-lower/src/analysis/cell_borrows/unproven.rs (#2490).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unproven-call-of-the-crate-passed",
    breaks: "`CellForm`'s proof alone passes a call of the crate's function while a guard is held, as if it ran std's code alone (#2490)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/unproven.rs",
    find: "            Called::Body(..) | Called::Bodies(_) => Some(Unproven::CrateFn),\n",
    replace: "            Called::Body(..) | Called::Bodies(_) => None,\n",
    tests: ["test/cell-form-shadow.test.ts", "-t", "the summaries prove quiet"],
  },
  {
    name: "unproven-cell-call-runs-nothing",
    breaks: "`CellForm`'s proof alone passes a cell's own call while a guard is held, a `with_borrow_mut` whose closure runs the crate's code (#2490)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/unproven.rs",
    find: "            Called::Cell(_) => self.std_runs_unproven(recognition, (def_id, args), held),\n",
    replace: "            Called::Cell(_) => None,\n",
    tests: ["test/cell-form-shadow.test.ts", "-t", "nothing can contest"],
  },
];
